//! Differential tests for the lowest-level exported entry point,
//! `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)`.
//!
//! This is the only way the `static stbds_siphash_bytes` in the C is reachable from
//! outside the translation unit, so exercising it covers the whole hash core:
//! the 8-byte main loop, the fall-through tail switch for every `len % 8`, and the
//! finalisation rounds.

mod common;

use common::{hex, libs, Rng};
use std::ffi::c_void;

/// Call both `.so` exports on the same buffer/seed and require identical results.
fn check(buf: &[u8], seed: usize) {
    let (c_fn, rust_fn) = libs().hash_bytes();

    // Each side gets its own copy so an accidental write by either is visible.
    let mut c_buf = buf.to_vec();
    let mut rust_buf = buf.to_vec();

    let c_out = unsafe { c_fn(c_buf.as_mut_ptr() as *mut c_void, buf.len(), seed) };
    let rust_out = unsafe { rust_fn(rust_buf.as_mut_ptr() as *mut c_void, buf.len(), seed) };

    assert_eq!(
        c_out,
        rust_out,
        "stbds_hash_bytes mismatch: len={} seed={:#x} data={}\n  C    = {:#018x}\n  Rust = {:#018x}",
        buf.len(),
        seed,
        hex(buf),
        c_out,
        rust_out
    );
    assert_eq!(
        c_buf,
        rust_buf,
        "input buffer was mutated differently: len={} seed={:#x}",
        buf.len(),
        seed
    );
}

#[test]
fn zero_length() {
    // C reads nothing when len == 0; the pointer is never dereferenced.
    check(&[], 0);
    check(&[], 1);
    check(&[], usize::MAX);
}

#[test]
fn all_lengths_ascending_bytes() {
    // Mirrors the buffer `siphash()` builds, but sweeps well past 64 bytes so the
    // main loop runs many times and `len << 56` overflows for len >= 256.
    let mut mem = [0u8; 600];
    for (i, b) in mem.iter_mut().enumerate() {
        *b = i as u8;
    }
    for len in 0..=mem.len() {
        check(&mem[..len], 0);
    }
}

#[test]
fn all_lengths_high_bit_bytes() {
    // Every byte >= 0x80: forces the sign-extending `d[3] << 24` / `d[7] << 24`
    // paths in both the main loop and `case 4:` of the tail switch.
    let mut mem = [0xffu8; 300];
    for len in 0..=mem.len() {
        check(&mem[..len], 0);
    }
    // A pattern where only the byte-3/byte-7 lanes have the high bit set.
    for (i, b) in mem.iter_mut().enumerate() {
        *b = if i % 4 == 3 { 0x80 | (i as u8) } else { (i as u8) & 0x7f };
    }
    for len in 0..=mem.len() {
        check(&mem[..len], 0);
    }
}

#[test]
fn tail_switch_every_remainder_and_high_bits() {
    // For each remainder 0..8, walk all 2^8 values through each tail byte position so
    // both the sign-extended and plain cases of every `case N:` arm are hit.
    for rem in 0..8usize {
        let len = 8 + rem; // one full main-loop iteration, then `rem` tail bytes
        for pos in 0..len {
            for v in 0..=255u8 {
                let mut buf = [0x5au8; 15];
                buf[pos] = v;
                check(&buf[..len], 0);
            }
        }
    }
}

#[test]
fn seed_variations() {
    let data: Vec<u8> = (0u16..137).map(|i| (i as u8).wrapping_mul(31)).collect();
    let mut seeds: Vec<usize> = vec![
        0,
        1,
        2,
        0xff,
        0x100,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1usize << (usize::BITS - 1),
        0x0706_0504_0302_0100,
        0x0f0e_0d0c_0b0a_0908,
        0xdead_beef_cafe_babe,
    ];
    for bit in 0..usize::BITS {
        seeds.push(1usize << bit);
        seeds.push(!(1usize << bit));
    }

    for &seed in &seeds {
        for len in [0usize, 1, 3, 7, 8, 9, 15, 16, 33, 64, 137] {
            check(&data[..len.min(data.len())], seed);
        }
    }
}

#[test]
fn randomised() {
    let mut rng = Rng(0xC0FF_EE12_3456_789A);
    let mut buf = [0u8; 512];
    for _ in 0..4000 {
        let len = (rng.next_u64() as usize) % (buf.len() + 1);
        rng.fill(&mut buf[..len]);
        let seed = rng.next_u64() as usize;
        check(&buf[..len], seed);
    }
}

#[test]
fn unaligned_and_offset_buffers() {
    // The C casts `void *` straight to `unsigned char *` and reads byte-wise, so any
    // alignment is legal. Slide the window through an over-allocated buffer.
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    let mut backing = vec![0u8; 200];
    rng.fill(&mut backing);
    for off in 0..16usize {
        for len in 0..=80usize {
            check(&backing[off..off + len], 0xa5a5_a5a5_a5a5_a5a5);
        }
    }
}

#[test]
fn exhaustive_short_buffers() {
    // Every possible buffer of length 0, 1 and 2 (65_793 cases) with a couple of seeds.
    for seed in [0usize, 0xdead_beef, usize::MAX] {
        check(&[], seed);
        for a in 0..=255u8 {
            check(&[a], seed);
        }
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                check(&[a, b], seed);
            }
        }
    }
}

#[test]
fn exhaustive_tail_byte_values_per_remainder() {
    // For remainders 1..8 fill the tail with a single repeated value across the whole
    // byte range, so each fall-through arm sees 0x00..0xff including the 0x80+ cases.
    for rem in 1..8usize {
        for v in 0..=255u8 {
            let mut buf = vec![0u8; 8 + rem];
            for b in buf[8..].iter_mut() {
                *b = v;
            }
            check(&buf, 0);
            for b in buf[..8].iter_mut() {
                *b = v;
            }
            check(&buf, 0);
        }
    }
}

#[test]
fn heavy_randomised_short() {
    // Dense fuzz over the length range where the tail switch dominates.
    let mut rng = Rng(0x5EED_0000_1111_2222);
    let mut buf = [0u8; 40];
    for _ in 0..150_000 {
        let len = (rng.next_u64() as usize) % (buf.len() + 1);
        rng.fill(&mut buf[..len]);
        let seed = rng.next_u64() as usize;
        check(&buf[..len], seed);
    }
}

#[test]
fn heavy_randomised_long() {
    // Lengths that straddle the `len << 56` truncation point (256) and beyond.
    let mut rng = Rng(0xABCD_0000_9999_8888);
    let mut buf = vec![0u8; 2048];
    for _ in 0..3_000 {
        let len = (rng.next_u64() as usize) % (buf.len() + 1);
        rng.fill(&mut buf[..len]);
        let seed = rng.next_u64() as usize;
        check(&buf[..len], seed);
    }
    // And a deterministic sweep over every length either side of 256.
    rng.fill(&mut buf);
    for len in 240..=272usize {
        check(&buf[..len], 0);
    }
}
