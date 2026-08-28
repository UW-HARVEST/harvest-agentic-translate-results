//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so`s via `libloading` and compares
//! `stbds_hash_bytes` / `siphash` outputs byte-for-byte across many randomized
//! inputs from a fixed-seed PRNG.

mod common;

use common::*;
use std::ffi::c_void;

/// Number of randomized samples per (len, shape) point.
const SAMPLES: usize = 64;

// ---------------------------------------------------------------------------
// Helpers for the byte-value shapes that drive the sign-extension paths.
// ---------------------------------------------------------------------------

/// Force byte `idx` of every 8-byte block to have its high bit set (or clear).
fn force_high_bit(buf: &mut [u8], idx_in_block: usize, set: bool) {
    let mut i = idx_in_block;
    while i < buf.len() {
        if set {
            buf[i] |= 0x80;
        } else {
            buf[i] &= 0x7f;
        }
        i += 8;
    }
}

/// Run `SAMPLES` randomized trials at a fixed `len` with an optional byte-shape
/// transform applied to the buffer.
fn sweep_len<F: Fn(&mut [u8])>(len: usize, ctx: &str, rng: &mut Rng, shape: F) {
    for s in 0..SAMPLES {
        let mut buf = vec![0u8; len.max(1) + 8];
        rng.fill(&mut buf);
        shape(&mut buf[..len]);
        let seed = rng.seed_value();
        diff_hash(&buf, len, seed, &format!("{ctx} sample={s}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 1-4: N=0 (no whole blocks), tail residue 0..3
// ---------------------------------------------------------------------------

#[test]
fn row01_len0_seed_sweep() {
    let mut rng = Rng::new(PRNG_SEED ^ 1);
    let buf = vec![0xABu8; 16];
    for seed in [0usize, 1, usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 63] {
        diff_hash(&buf, 0, seed, &format!("row01 len=0 seed={seed:#x}"));
    }
    for s in 0..512 {
        let seed = rng.seed_value();
        diff_hash(&buf, 0, seed, &format!("row01 len=0 random seed sample={s}"));
    }
}

#[test]
fn row02_len1() {
    let mut rng = Rng::new(PRNG_SEED ^ 2);
    sweep_len(1, "row02 len=1", &mut rng, |_| {});
    // exhaustive over the single byte, several seeds
    for b in 0u16..=255 {
        let buf = [b as u8; 1];
        for seed in [0usize, usize::MAX, 0x1234_5678_9abc_def0] {
            diff_hash(&buf, 1, seed, &format!("row02 exhaustive byte={b:#04x} seed={seed:#x}"));
        }
    }
}

#[test]
fn row03_len2() {
    let mut rng = Rng::new(PRNG_SEED ^ 3);
    sweep_len(2, "row03 len=2", &mut rng, |_| {});
}

#[test]
fn row04_len3() {
    let mut rng = Rng::new(PRNG_SEED ^ 4);
    sweep_len(3, "row04 len=3", &mut rng, |_| {});
}

// ---------------------------------------------------------------------------
// Rows 5-7: len==4 -- the tail `case 4:` `d[3] << 24` int-overflow sign-extension
// ---------------------------------------------------------------------------

#[test]
fn row05_len4_random() {
    let mut rng = Rng::new(PRNG_SEED ^ 5);
    sweep_len(4, "row05 len=4 random", &mut rng, |_| {});
}

#[test]
fn row06_len4_tail_byte3_high_bit_set() {
    let mut rng = Rng::new(PRNG_SEED ^ 6);
    sweep_len(4, "row06 len=4 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
    // Exhaustive over d[3] in 0x80..=0xff with random low bytes.
    for hi in 0x80u16..=0xff {
        let mut buf = [0u8; 4];
        rng.fill(&mut buf);
        buf[3] = hi as u8;
        diff_hash(&buf, 4, 0, &format!("row06 exhaustive d[3]={hi:#04x}"));
    }
}

#[test]
fn row07_len4_tail_byte3_high_bit_clear() {
    let mut rng = Rng::new(PRNG_SEED ^ 7);
    sweep_len(4, "row07 len=4 d[3]<0x80", &mut rng, |b| force_high_bit(b, 3, false));
    for hi in 0x00u16..=0x7f {
        let mut buf = [0u8; 4];
        rng.fill(&mut buf);
        buf[3] = hi as u8;
        diff_hash(&buf, 4, 0, &format!("row07 exhaustive d[3]={hi:#04x}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 8-10: len==5,6,7 -- tail arms 5,6,7 falling through into 4,3,2,1
// ---------------------------------------------------------------------------

#[test]
fn row08_len5() {
    let mut rng = Rng::new(PRNG_SEED ^ 8);
    sweep_len(5, "row08 len=5 random", &mut rng, |_| {});
    sweep_len(5, "row08 len=5 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
    sweep_len(5, "row08 len=5 d[3]<0x80", &mut rng, |b| force_high_bit(b, 3, false));
}

#[test]
fn row09_len6() {
    let mut rng = Rng::new(PRNG_SEED ^ 9);
    sweep_len(6, "row09 len=6 random", &mut rng, |_| {});
    sweep_len(6, "row09 len=6 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
    sweep_len(6, "row09 len=6 d[3]<0x80", &mut rng, |b| force_high_bit(b, 3, false));
}

#[test]
fn row10_len7_all_tail_arms() {
    let mut rng = Rng::new(PRNG_SEED ^ 10);
    sweep_len(7, "row10 len=7 random", &mut rng, |_| {});
    sweep_len(7, "row10 len=7 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
    sweep_len(7, "row10 len=7 d[3]<0x80", &mut rng, |b| force_high_bit(b, 3, false));
    // Exhaustive on d[6], d[5], d[4] high bits (the `(size_t)` casted arms).
    for mask in 0u8..8 {
        let mut buf = [0u8; 7];
        rng.fill(&mut buf);
        buf[6] = if mask & 1 != 0 { 0xff } else { 0x01 };
        buf[5] = if mask & 2 != 0 { 0xff } else { 0x01 };
        buf[4] = if mask & 4 != 0 { 0xff } else { 0x01 };
        diff_hash(&buf, 7, 0, &format!("row10 tail-hi mask={mask}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 11-15: len==8 -- exactly one main-loop block, both `cltq` halves
// ---------------------------------------------------------------------------

#[test]
fn row11_len8_random() {
    let mut rng = Rng::new(PRNG_SEED ^ 11);
    sweep_len(8, "row11 len=8 random", &mut rng, |_| {});
}

#[test]
fn row12_len8_block_byte3_high() {
    let mut rng = Rng::new(PRNG_SEED ^ 12);
    sweep_len(8, "row12 len=8 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
}

#[test]
fn row13_len8_block_byte7_high() {
    let mut rng = Rng::new(PRNG_SEED ^ 13);
    sweep_len(8, "row13 len=8 d[7]>=0x80", &mut rng, |b| force_high_bit(b, 7, true));
}

#[test]
fn row14_len8_both_bytes_high() {
    let mut rng = Rng::new(PRNG_SEED ^ 14);
    sweep_len(8, "row14 len=8 d[3]&d[7]>=0x80", &mut rng, |b| {
        force_high_bit(b, 3, true);
        force_high_bit(b, 7, true);
    });
    // Full 2x2 matrix of the two sign-extension triggers, exhaustive on the
    // extreme byte values.
    for (b3, b7) in [(0x7fu8, 0x7fu8), (0x80, 0x7f), (0x7f, 0x80), (0x80, 0x80), (0xff, 0xff)] {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        buf[3] = b3;
        buf[7] = b7;
        for seed in [0usize, usize::MAX, 0xdead_beef_cafe_babe] {
            diff_hash(&buf, 8, seed, &format!("row14 matrix d3={b3:#04x} d7={b7:#04x}"));
        }
    }
}

#[test]
fn row15_len8_neither_byte_high() {
    let mut rng = Rng::new(PRNG_SEED ^ 15);
    sweep_len(8, "row15 len=8 neither high", &mut rng, |b| {
        force_high_bit(b, 3, false);
        force_high_bit(b, 7, false);
    });
}

// ---------------------------------------------------------------------------
// Rows 16-19: one/two/many blocks crossed with every tail arm
// ---------------------------------------------------------------------------

#[test]
fn row16_len9_to_15_one_block_all_tails() {
    let mut rng = Rng::new(PRNG_SEED ^ 16);
    for len in 9..=15 {
        sweep_len(len, &format!("row16 len={len} random"), &mut rng, |_| {});
        sweep_len(len, &format!("row16 len={len} d[3]>=0x80"), &mut rng, |b| {
            force_high_bit(b, 3, true)
        });
        sweep_len(len, &format!("row16 len={len} d[7]>=0x80"), &mut rng, |b| {
            force_high_bit(b, 7, true)
        });
    }
}

#[test]
fn row17_len16_two_blocks() {
    let mut rng = Rng::new(PRNG_SEED ^ 17);
    sweep_len(16, "row17 len=16 random", &mut rng, |_| {});
    sweep_len(16, "row17 len=16 d[3]>=0x80", &mut rng, |b| force_high_bit(b, 3, true));
    sweep_len(16, "row17 len=16 d[7]>=0x80", &mut rng, |b| force_high_bit(b, 7, true));
}

#[test]
fn row18_len17_to_23_two_blocks_all_tails() {
    let mut rng = Rng::new(PRNG_SEED ^ 18);
    for len in 17..=23 {
        sweep_len(len, &format!("row18 len={len} random"), &mut rng, |_| {});
        sweep_len(len, &format!("row18 len={len} both high"), &mut rng, |b| {
            force_high_bit(b, 3, true);
            force_high_bit(b, 7, true);
        });
    }
}

#[test]
fn row19_len_sweep_0_to_264() {
    let mut rng = Rng::new(PRNG_SEED ^ 19);
    for len in 0..=264usize {
        for s in 0..8 {
            let mut buf = vec![0u8; len + 8];
            rng.fill(&mut buf);
            let seed = rng.seed_value();
            diff_hash(&buf, len, seed, &format!("row19 len={len} sample={s}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 20-22: degenerate / boundary byte patterns
// ---------------------------------------------------------------------------

#[test]
fn row20_all_zero_bytes() {
    let buf = vec![0x00u8; 80];
    for len in 0..=64usize {
        for seed in [0usize, usize::MAX, 1, usize::MAX / 2] {
            diff_hash(&buf, len, seed, &format!("row20 zeros len={len} seed={seed:#x}"));
        }
    }
}

#[test]
fn row21_all_ff_bytes() {
    let buf = vec![0xFFu8; 80];
    for len in 0..=64usize {
        for seed in [0usize, usize::MAX, 1, usize::MAX / 2] {
            diff_hash(&buf, len, seed, &format!("row21 0xff len={len} seed={seed:#x}"));
        }
    }
}

#[test]
fn row22_high_bit_boundary_pattern() {
    // Alternating 0x7f/0x80 straddles the int-overflow boundary at every
    // offset; also test the two phase shifts and a 0x80-everywhere buffer.
    for phase in 0..2usize {
        let buf: Vec<u8> =
            (0..80).map(|i| if (i + phase) % 2 == 0 { 0x7f } else { 0x80 }).collect();
        for len in 0..=64usize {
            for seed in [0usize, usize::MAX] {
                diff_hash(&buf, len, seed, &format!("row22 phase={phase} len={len}"));
            }
        }
    }
    let buf = vec![0x80u8; 80];
    for len in 0..=64usize {
        diff_hash(&buf, len, 0, &format!("row22 all-0x80 len={len}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25: seed axis
// ---------------------------------------------------------------------------

#[test]
fn row23_seed_zero_fixed() {
    let mut rng = Rng::new(PRNG_SEED ^ 23);
    for len in 0..=72usize {
        for _ in 0..8 {
            let mut buf = vec![0u8; len + 8];
            rng.fill(&mut buf);
            diff_hash(&buf, len, 0, &format!("row23 seed=0 len={len}"));
        }
    }
}

#[test]
fn row24_seed_max_fixed() {
    let mut rng = Rng::new(PRNG_SEED ^ 24);
    for len in 0..=72usize {
        for _ in 0..8 {
            let mut buf = vec![0u8; len + 8];
            rng.fill(&mut buf);
            diff_hash(&buf, len, usize::MAX, &format!("row24 seed=MAX len={len}"));
        }
    }
}

#[test]
fn row25_property_20k_random() {
    let mut rng = Rng::new(PRNG_SEED ^ 25);
    let mut buf = vec![0u8; 300];
    for s in 0..20_000usize {
        rng.fill(&mut buf);
        let len = rng.below(buf.len() + 1);
        let seed = rng.seed_value();
        diff_hash(&buf, len, seed, &format!("row25 property sample={s}"));
    }
}

// ---------------------------------------------------------------------------
// Row 26: alignment axis
// ---------------------------------------------------------------------------

#[test]
fn row26_misaligned_pointers() {
    let mut rng = Rng::new(PRNG_SEED ^ 26);
    let mut backing = vec![0u8; 64];
    for trial in 0..32 {
        rng.fill(&mut backing);
        for off in 0..8usize {
            for len in 0..=32usize {
                let mut a = backing.clone();
                let mut b = backing.clone();
                let seed = rng.seed_value();
                let cv = unsafe {
                    (c_lib().hash_bytes)(a[off..].as_mut_ptr() as *mut c_void, len, seed)
                };
                let rv = unsafe {
                    (rust_lib().hash_bytes)(b[off..].as_mut_ptr() as *mut c_void, len, seed)
                };
                assert_eq!(
                    cv, rv,
                    "row26 misaligned divergence trial={trial} off={off} len={len} \
                     seed={seed:#x}\n  C={cv:#018x} RUST={rv:#018x}"
                );
                // A misaligned read must agree with the same bytes copied to an
                // aligned buffer -- i.e. alignment is genuinely irrelevant.
                let aligned = backing[off..off + len].to_vec();
                let ref_v = diff_hash(&aligned, len, seed, "row26 aligned reference");
                assert_eq!(
                    cv, ref_v,
                    "row26 alignment changed the result: off={off} len={len}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27: large input
// ---------------------------------------------------------------------------

#[test]
fn row27_one_mib_input() {
    let mut rng = Rng::new(PRNG_SEED ^ 27);
    let mut buf = vec![0u8; 1 << 20];
    rng.fill(&mut buf);
    for seed in [0usize, 1, usize::MAX, 0x0f0f_0f0f_0f0f_0f0f] {
        diff_hash(&buf, buf.len(), seed, &format!("row27 1MiB seed={seed:#x}"));
    }
    // A few odd lengths near the end so the tail arm is exercised at scale.
    for delta in 0..8usize {
        let len = buf.len() - delta;
        diff_hash(&buf, len, 0, &format!("row27 1MiB len={len}"));
    }
}

// ---------------------------------------------------------------------------
// Row 28: purity / no hidden state
// ---------------------------------------------------------------------------

#[test]
fn row28_determinism_and_interleaving() {
    let mut rng = Rng::new(PRNG_SEED ^ 28);
    let mut buf = vec![0u8; 100];
    rng.fill(&mut buf);
    let len = 57usize;
    let seed = 0x1234_5678_9abc_def0usize;

    let mut work = buf.clone();
    let first_c = unsafe { (c_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, len, seed) };
    let first_r = unsafe { (rust_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, len, seed) };
    assert_eq!(first_c, first_r, "row28 initial call divergence");

    // Interleave the two implementations many times; results must never drift.
    for i in 0..1000 {
        let c = unsafe { (c_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, len, seed) };
        let r = unsafe { (rust_lib().hash_bytes)(work.as_mut_ptr() as *mut c_void, len, seed) };
        assert_eq!(c, first_c, "row28 C not deterministic at iteration {i}");
        assert_eq!(r, first_r, "row28 RUST not deterministic at iteration {i}");
    }
    assert_eq!(work, buf, "row28 buffer was mutated by the hash");
}
