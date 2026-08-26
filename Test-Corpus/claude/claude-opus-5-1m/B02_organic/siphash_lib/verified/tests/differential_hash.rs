//! Phase B — valid-path differential tests for the LOW-LEVEL entry point
//! `stbds_hash_bytes`, one test per row of `CONFIGS.md` (rows 1..=30).
//!
//! Both implementations are invoked only through their exported symbols in their
//! respective `.so` files.

mod common;

use common::{diff_hash, diff_hash_raw, impls, seed_corpus, Rng};
use std::ffi::c_void;

/// Allocates an 8-byte-aligned scratch buffer of at least `bytes` bytes.
fn aligned_buf(bytes: usize) -> Vec<u64> {
    vec![0u64; bytes / 8 + 2]
}

// ---------------------------------------------------------------------------
// Rows 1-3: len == 0
// ---------------------------------------------------------------------------

#[test]
fn row01_len0_seed0() {
    let mut rng = Rng::new(0x0101_0101);
    let mut buf = [0u8; 64];
    let mut first = None;
    for t in 0..512 {
        rng.fill(&mut buf);
        let h = diff_hash(&mut buf, 0, 0, &format!("row01 t={t}"));
        // The buffer must be irrelevant when len == 0.
        match first {
            None => first = Some(h),
            Some(f) => assert_eq!(
                f, h,
                "row01: len=0 result depended on buffer contents (C-side invariant)"
            ),
        }
    }
}

#[test]
fn row02_len0_seed_max() {
    let mut rng = Rng::new(0x0202_0202);
    let mut buf = [0u8; 64];
    for t in 0..256 {
        rng.fill(&mut buf);
        diff_hash(&mut buf, 0, usize::MAX, &format!("row02 t={t}"));
        diff_hash(&mut buf, 0, usize::MAX - 1, &format!("row02b t={t}"));
    }
}

#[test]
fn row03_len0_seed_random() {
    let mut rng = Rng::new(0x0303_0303);
    let mut buf = [0u8; 64];
    rng.fill(&mut buf);
    for s in seed_corpus(&mut rng, 512) {
        diff_hash(&mut buf, 0, s, &format!("row03 seed={s:#x}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 4-12: no full block, tail cases 1..7
// ---------------------------------------------------------------------------

/// Shared driver for a fixed short length: randomized bytes x randomized seeds,
/// with `tweak` applied to the buffer to force a specific sign-extension class.
fn short_len_row(row: &str, len: usize, seed0: u64, trials: usize, tweak: fn(&mut [u8])) {
    let mut rng = Rng::new(seed0);
    let mut buf = [0u8; 16];
    for t in 0..trials {
        rng.fill(&mut buf);
        tweak(&mut buf);
        let seed = if t % 4 == 0 { 0 } else { rng.next_usize() };
        diff_hash(&mut buf, len, seed, &format!("{row} t={t}"));
    }
    // Plus every extreme byte pattern at this length.
    for fill in [0x00u8, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        let mut b = [fill; 16];
        tweak(&mut b);
        for s in [0usize, 1, usize::MAX, 1usize << 63] {
            diff_hash(&mut b, len, s, &format!("{row} fill={fill:#04x} seed={s:#x}"));
        }
    }
}

fn no_tweak(_: &mut [u8]) {}
fn clear_b3(b: &mut [u8]) {
    b[3] &= 0x7f;
}
fn set_b3(b: &mut [u8]) {
    b[3] |= 0x80;
}
fn clear_b3_b7(b: &mut [u8]) {
    b[3] &= 0x7f;
    b[7] &= 0x7f;
}
fn clear_b3_set_b7(b: &mut [u8]) {
    b[3] &= 0x7f;
    b[7] |= 0x80;
}

#[test]
fn row04_len1() {
    // Exhaustive over all 256 byte values, then randomized seeds.
    let mut rng = Rng::new(0x0404_0404);
    for v in 0u16..=255 {
        let mut b = [v as u8; 1];
        diff_hash(&mut b, 1, 0, &format!("row04 exhaustive v={v}"));
    }
    for s in seed_corpus(&mut rng, 64) {
        for v in [0u8, 1, 0x7f, 0x80, 0xff] {
            let mut b = [v];
            diff_hash(&mut b, 1, s, &format!("row04 v={v:#04x} seed={s:#x}"));
        }
    }
    short_len_row("row04", 1, 0x1404, 512, no_tweak);
}

#[test]
fn row05_len2() {
    short_len_row("row05", 2, 0x0505_0505, 1024, no_tweak);
}

#[test]
fn row06_len3() {
    short_len_row("row06", 3, 0x0606_0606, 1024, no_tweak);
    // force d[2] >= 0x80 (case 3 with the high byte set)
    short_len_row("row06hi", 3, 0x1606, 512, |b| b[2] |= 0x80);
}

#[test]
fn row07_len4_no_signext() {
    short_len_row("row07", 4, 0x0707_0707, 1024, clear_b3);
}

#[test]
fn row08_len4_signext() {
    short_len_row("row08", 4, 0x0808_0808, 1024, set_b3);
}

#[test]
fn row09_len5() {
    short_len_row("row09", 5, 0x0909_0909, 512, no_tweak);
    short_len_row("row09lo", 5, 0x1909, 512, clear_b3);
    short_len_row("row09hi", 5, 0x2909, 512, set_b3);
}

#[test]
fn row10_len6() {
    short_len_row("row10", 6, 0x0a0a_0a0a, 512, no_tweak);
    short_len_row("row10lo", 6, 0x1a0a, 512, clear_b3);
    short_len_row("row10hi", 6, 0x2a0a, 512, set_b3);
}

#[test]
fn row11_len7() {
    short_len_row("row11", 7, 0x0b0b_0b0b, 512, no_tweak);
    short_len_row("row11lo", 7, 0x1b0b, 512, clear_b3);
    short_len_row("row11hi", 7, 0x2b0b, 512, set_b3);
}

#[test]
fn row12_len7_extremes() {
    let mut rng = Rng::new(0x0c0c_0c0c);
    for fill in [0x00u8, 0xff] {
        let mut b = [fill; 8];
        for s in seed_corpus(&mut rng, 32) {
            for len in 0..=7usize {
                diff_hash(
                    &mut b,
                    len,
                    s,
                    &format!("row12 fill={fill:#04x} len={len} seed={s:#x}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13-17: exactly one block
// ---------------------------------------------------------------------------

#[test]
fn row13_len8_random() {
    short_len_row("row13", 8, 0x0d0d_0d0d, 2048, no_tweak);
}

#[test]
fn row14_len8_no_signext() {
    short_len_row("row14", 8, 0x0e0e_0e0e, 1024, clear_b3_b7);
}

#[test]
fn row15_len8_low_signext() {
    short_len_row("row15", 8, 0x0f0f_0f0f, 1024, set_b3);
}

#[test]
fn row16_len8_high_signext() {
    short_len_row("row16", 8, 0x1010_1010, 1024, clear_b3_set_b7);
}

#[test]
fn row17_len8_extremes() {
    let mut rng = Rng::new(0x1111_1111);
    for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
        let mut b = [fill; 8];
        for s in seed_corpus(&mut rng, 32) {
            diff_hash(&mut b, 8, s, &format!("row17 fill={fill:#04x} seed={s:#x}"));
        }
    }
    // Every single byte position set to 0x80 with the rest zero, and to 0x00
    // with the rest 0xff — isolates each byte's contribution.
    for pos in 0..8usize {
        for (base, v) in [(0x00u8, 0x80u8), (0xffu8, 0x00u8), (0x00u8, 0xffu8)] {
            let mut b = [base; 8];
            b[pos] = v;
            for s in [0usize, 1, usize::MAX, 1 << 63] {
                diff_hash(
                    &mut b,
                    8,
                    s,
                    &format!("row17 pos={pos} base={base:#04x} v={v:#04x} seed={s:#x}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 18-23: multiple blocks / long inputs
// ---------------------------------------------------------------------------

fn len_range_row(row: &str, lo: usize, hi: usize, seed0: u64, trials_per_len: usize) {
    let mut rng = Rng::new(seed0);
    let mut buf = vec![0u8; hi + 8];
    for len in lo..=hi {
        for t in 0..trials_per_len {
            rng.fill(&mut buf);
            let seed = match t % 5 {
                0 => 0,
                1 => usize::MAX,
                2 => 1usize << 63,
                3 => 1,
                _ => rng.next_usize(),
            };
            diff_hash(&mut buf, len, seed, &format!("{row} len={len} t={t}"));
        }
        // Extremes at this length too.
        for fill in [0x00u8, 0x80, 0xff] {
            let mut b = vec![fill; hi + 8];
            diff_hash(&mut b, len, 0, &format!("{row} len={len} fill={fill:#04x}"));
            diff_hash(
                &mut b,
                len,
                usize::MAX,
                &format!("{row} len={len} fill={fill:#04x} seedmax"),
            );
        }
    }
}

#[test]
fn row18_len9_15() {
    len_range_row("row18", 9, 15, 0x1212_1212, 128);
}

#[test]
fn row19_len16() {
    len_range_row("row19", 16, 16, 0x1313_1313, 1024);
}

#[test]
fn row20_len17_63() {
    len_range_row("row20", 17, 63, 0x1414_1414, 24);
}

#[test]
fn row21_len64_255() {
    len_range_row("row21", 64, 255, 0x1515_1515, 8);
}

#[test]
fn row22_len_ge_256() {
    // len >= 256 makes `len << 56` keep only `len & 0xFF`.
    let mut rng = Rng::new(0x1616_1616);
    let mut buf = vec![0u8; 1100];
    // Every length across a full 256-wide window so all `len & 0xFF` values and
    // all tail remainders occur.
    for len in 256..=520usize {
        rng.fill(&mut buf);
        let seed = if len % 3 == 0 { 0 } else { rng.next_usize() };
        diff_hash(&mut buf, len, seed, &format!("row22 len={len}"));
    }
    for len in [768usize, 769, 1023, 1024] {
        for _ in 0..8 {
            rng.fill(&mut buf);
            let s = rng.next_usize();
            diff_hash(&mut buf, len, s, &format!("row22b len={len} seed={s:#x}"));
        }
    }
}

#[test]
fn row23_len_multiple_of_8_large() {
    let mut rng = Rng::new(0x1717_1717);
    let mut buf = vec![0u8; 4096];
    for len in [512usize, 1024, 2048, 4096] {
        for t in 0..16 {
            rng.fill(&mut buf);
            let seed = match t % 4 {
                0 => 0,
                1 => usize::MAX,
                2 => 1usize << 63,
                _ => rng.next_usize(),
            };
            diff_hash(&mut buf, len, seed, &format!("row23 len={len} t={t}"));
        }
    }
    // All-0x00 and all-0xff at max length.
    for fill in [0x00u8, 0xff] {
        let mut b = vec![fill; 4096];
        for s in [0usize, usize::MAX] {
            diff_hash(&mut b, 4096, s, &format!("row23 fill={fill:#04x} seed={s:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24: unaligned pointers
// ---------------------------------------------------------------------------

#[test]
fn row24_unaligned_pointer() {
    let mut rng = Rng::new(0x1818_1818);
    let mut backing = aligned_buf(256);
    let base = backing.as_mut_ptr() as *mut u8;
    assert_eq!(base as usize % 8, 0, "backing store must be 8-byte aligned");

    for off in 0..8usize {
        for len in 0..=40usize {
            for t in 0..6 {
                // Refill the whole backing store through the raw pointer.
                unsafe {
                    for i in 0..256usize {
                        *base.add(i) = rng.next_u8();
                    }
                }
                let seed = match t % 3 {
                    0 => 0,
                    1 => usize::MAX,
                    _ => rng.next_usize(),
                };
                let p = unsafe { base.add(off) } as *mut c_void;
                diff_hash_raw(p, len, seed, &format!("row24 off={off} len={len} t={t}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 25-27: seed axis
// ---------------------------------------------------------------------------

fn seed_row(row: &str, seed: usize, seed0: u64) {
    let mut rng = Rng::new(seed0);
    let mut buf = vec![0u8; 80];
    for len in 0..=72usize {
        for t in 0..6 {
            rng.fill(&mut buf);
            diff_hash(&mut buf, len, seed, &format!("{row} len={len} t={t}"));
        }
        for fill in [0x00u8, 0x80, 0xff] {
            let mut b = vec![fill; 80];
            diff_hash(&mut b, len, seed, &format!("{row} len={len} fill={fill:#04x}"));
        }
    }
}

#[test]
fn row25_seed_max_all_lens() {
    seed_row("row25", usize::MAX, 0x1919_1919);
    seed_row("row25b", usize::MAX - 1, 0x2919_1919);
}

#[test]
fn row26_seed_highbit_all_lens() {
    seed_row("row26", 1usize << 63, 0x1a1a_1a1a);
    seed_row("row26b", 1usize, 0x2a1a_1a1a);
}

#[test]
fn row27_seed_single_bit_sweep() {
    let mut rng = Rng::new(0x1b1b_1b1b);
    let mut buf = vec![0u8; 48];
    rng.fill(&mut buf);
    for k in 0..64u32 {
        let seed = 1usize << k;
        for len in [0usize, 1, 3, 7, 8, 9, 15, 16, 17, 33, 40] {
            diff_hash(&mut buf, len, seed, &format!("row27 k={k} len={len}"));
            // also the complement of that single-bit seed
            diff_hash(&mut buf, len, !seed, &format!("row27c k={k} len={len}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 28: single-bit data avalanche
// ---------------------------------------------------------------------------

#[test]
fn row28_single_bit_data_avalanche() {
    for len in 1..=32usize {
        for byte in 0..len {
            for bit in 0..8u32 {
                let mut b = vec![0u8; 32];
                b[byte] = 1u8 << bit;
                diff_hash(&mut b, len, 0, &format!("row28 len={len} byte={byte} bit={bit}"));
                // and the inverted buffer (single cleared bit in all-ones)
                let mut c = vec![0xffu8; 32];
                c[byte] = !(1u8 << bit);
                diff_hash(
                    &mut c,
                    len,
                    0,
                    &format!("row28inv len={len} byte={byte} bit={bit}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 29: exactly what `siphash` feeds in, but through the low-level API
// ---------------------------------------------------------------------------

#[test]
fn row29_sequential_pattern_all_lens() {
    let mut rng = Rng::new(0x1d1d_1d1d);
    let mut inits: Vec<i32> = vec![0, 1, 42, -1, -128, 127, 128, 255, i32::MIN, i32::MAX];
    for _ in 0..24 {
        inits.push(rng.next_i32());
    }
    for init in inits {
        // Replicates `siphash`'s buffer construction: mem[i] = (unsigned char)z, z++.
        let mut mem = [0u8; 64];
        let mut z: i32 = init;
        for i in 0..64usize {
            mem[i] = z as u8;
            z = z.wrapping_add(1);
        }
        for len in 0..64usize {
            diff_hash(&mut mem, len, 0, &format!("row29 init={init} len={len}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 30: full length sweep with random data and random seeds
// ---------------------------------------------------------------------------

#[test]
fn row30_full_length_sweep() {
    let mut rng = Rng::new(0x1e1e_1e1e);
    let mut buf = vec![0u8; 560];
    for len in 0..=520usize {
        for t in 0..3 {
            rng.fill(&mut buf);
            let seed = match t {
                0 => 0,
                1 => usize::MAX,
                _ => rng.next_usize(),
            };
            diff_hash(&mut buf, len, seed, &format!("row30 len={len} t={t}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: both objects really are two distinct shared libraries
// ---------------------------------------------------------------------------

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let (c, r) = impls();
    assert_ne!(c.path, r.path, "C and Rust .so paths must differ");
    // Unless explicitly overridden (C_SO / RUST_SO, used to re-run the suite
    // against other builds), the objects must come from the expected places.
    if std::env::var_os("C_SO").is_none() {
        assert!(
            c.path.to_string_lossy().contains("c_src"),
            "C object should come from c_src/build: {}",
            c.path.display()
        );
    }
    if std::env::var_os("RUST_SO").is_none() {
        assert!(
            r.path.to_string_lossy().contains("target"),
            "Rust object should come from target/: {}",
            r.path.display()
        );
    }
    assert_ne!(
        c.hash_bytes as usize, r.hash_bytes as usize,
        "the two stbds_hash_bytes symbols must be different code"
    );
    assert_ne!(
        c.siphash as usize, r.siphash as usize,
        "the two siphash symbols must be different code"
    );
}
