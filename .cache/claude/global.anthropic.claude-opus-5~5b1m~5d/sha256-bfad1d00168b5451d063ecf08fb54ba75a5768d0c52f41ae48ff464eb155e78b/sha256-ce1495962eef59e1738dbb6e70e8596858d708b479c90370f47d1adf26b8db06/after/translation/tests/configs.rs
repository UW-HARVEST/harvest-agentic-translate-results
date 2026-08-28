//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through their exported
//! `max_size_frame` symbol and asserts byte-identical results across many
//! randomized inputs (fixed seed).

mod common;

use common::{impls, Rng, DRAWS, SEED, TYPICAL_BITDEPTHS, TYPICAL_BLOCKSIZES};

const MAX: u32 = u32::MAX;

/// Distinct seed per row so rows don't share the same draw sequence.
fn rng(row: u64) -> Rng {
    Rng::new(SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

// --------------------------------------------------------------- C1 .. C7
// Stereo branch (channels == 2): t1 is annihilated, t2 and t3 are live,
// and `bitdepth + (bitdepth != 32)` is observable here.

#[test]
fn c1_stereo_bitdepth_32() {
    let f = impls();
    let mut r = rng(1);
    for _ in 0..DRAWS {
        f.assert_eq(r.range(1, 65535), 2, 32);
    }
}

#[test]
fn c2_stereo_typical_bitdepths() {
    let f = impls();
    let mut r = rng(2);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), 2, bd);
    }
}

#[test]
fn c3_stereo_bitdepth_zero() {
    let f = impls();
    let mut r = rng(3);
    for _ in 0..DRAWS {
        f.assert_eq(r.range(0, 65535), 2, 0);
    }
}

#[test]
fn c4_stereo_bitdepth_31_and_33() {
    let f = impls();
    let mut r = rng(4);
    for _ in 0..DRAWS {
        let bs = r.range(0, 65535);
        f.assert_eq(bs, 2, 31);
        f.assert_eq(bs, 2, 33);
        // also both sides of the boundary at full-u32 blocksizes
        let bs = r.next_u32();
        f.assert_eq(bs, 2, 31);
        f.assert_eq(bs, 2, 32);
        f.assert_eq(bs, 2, 33);
    }
}

#[test]
fn c5_stereo_bitdepth_max_wraps_plus_one() {
    let f = impls();
    let mut r = rng(5);
    for _ in 0..DRAWS {
        f.assert_eq(r.range(0, 65535), 2, MAX);
        f.assert_eq(r.next_u32(), 2, MAX);
    }
}

#[test]
fn c6_stereo_blocksize_zero() {
    let f = impls();
    let mut r = rng(6);
    for _ in 0..DRAWS {
        f.assert_eq(0, 2, r.next_u32());
    }
}

#[test]
fn c7_stereo_full_u32_blocksize_bitdepth() {
    let f = impls();
    let mut r = rng(7);
    for _ in 0..DRAWS {
        f.assert_eq(r.next_u32(), 2, r.next_u32());
    }
}

// -------------------------------------------------------------- C8 .. C10
// Mono branch (channels == 1): t2/t3 annihilated, only t1 = bs*bd*1 lives.

#[test]
fn c8_mono_bitdepth_32() {
    let f = impls();
    let mut r = rng(8);
    for _ in 0..DRAWS {
        f.assert_eq(r.range(1, 65535), 1, 32);
    }
}

#[test]
fn c9_mono_typical_bitdepths() {
    let f = impls();
    let mut r = rng(9);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), 1, bd);
    }
}

#[test]
fn c10_mono_bitdepth_max_plus_one_unused() {
    let f = impls();
    let mut r = rng(10);
    for _ in 0..DRAWS {
        f.assert_eq(r.range(0, 65535), 1, MAX);
        f.assert_eq(r.next_u32(), 1, MAX);
    }
}

// ------------------------------------------------------------- C11 .. C18
// The `channels` axis away from 0/1/2.

#[test]
fn c11_channels_zero_annihilates() {
    let f = impls();
    let mut r = rng(11);
    for _ in 0..DRAWS {
        // Must be 18 for every blocksize/bitdepth.
        let got = f.assert_eq(r.next_u32(), 0, r.next_u32());
        assert_eq!(got, 18, "channels=0 must annihilate t1 and yield 18");
    }
}

#[test]
fn c12_channels_three() {
    let f = impls();
    let mut r = rng(12);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), 3, bd);
    }
}

#[test]
fn c13_channels_4_to_8() {
    let f = impls();
    let mut r = rng(13);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), r.range(4, 8), bd);
    }
}

#[test]
fn c14_channels_nine_one_past_flac_max() {
    let f = impls();
    let mut r = rng(14);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), 9, bd);
        f.assert_eq(r.next_u32(), 9, r.next_u32());
    }
}

#[test]
fn c15_channels_10_to_255() {
    let f = impls();
    let mut r = rng(15);
    for _ in 0..DRAWS {
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(r.range(1, 65535), r.range(10, 255), bd);
    }
}

#[test]
fn c16_channels_256_to_65535() {
    let f = impls();
    let mut r = rng(16);
    for _ in 0..DRAWS {
        f.assert_eq(r.next_u32(), r.range(256, 65535), r.next_u32());
    }
}

#[test]
fn c17_channels_max_overflows_18_plus_ch() {
    let f = impls();
    let mut r = rng(17);
    for _ in 0..DRAWS {
        f.assert_eq(r.next_u32(), MAX, r.next_u32());
    }
}

#[test]
fn c18_channels_across_18_plus_ch_wrap_point() {
    let f = impls();
    let mut r = rng(18);
    for ch in (MAX - 17)..=MAX {
        for _ in 0..64 {
            f.assert_eq(r.next_u32(), ch, r.next_u32());
        }
        f.assert_eq(0, ch, 0);
        f.assert_eq(4096, ch, 16);
    }
    // and the same sweep on the stereo-adjacent low end
    for ch in 0..=20u32 {
        for _ in 0..64 {
            f.assert_eq(r.next_u32(), ch, r.next_u32());
        }
    }
}

// ------------------------------------------------------------- C19 .. C22
// The `blocksize` axis.

#[test]
fn c19_blocksize_zero_full_random_others() {
    let f = impls();
    let mut r = rng(19);
    for _ in 0..DRAWS {
        f.assert_eq(0, r.next_u32(), r.next_u32());
    }
}

#[test]
fn c20_blocksize_one_dense_small_grid() {
    let f = impls();
    for ch in 0..=8u32 {
        for bd in 0..=33u32 {
            f.assert_eq(1, ch, bd);
        }
    }
    let mut r = rng(20);
    for _ in 0..DRAWS {
        f.assert_eq(1, r.range(0, 8), r.range(0, 33));
    }
}

#[test]
fn c21_blocksize_65535_and_65536() {
    let f = impls();
    let mut r = rng(21);
    for _ in 0..DRAWS {
        let ch = r.range(1, 8);
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(65535, ch, bd);
        f.assert_eq(65536, ch, bd);
        f.assert_eq(65537, ch, bd);
    }
}

#[test]
fn c22_blocksize_huge() {
    let f = impls();
    let mut r = rng(22);
    let huge = [1u32 << 31, (1u32 << 31) - 1, (1u32 << 31) + 1, MAX, MAX - 1];
    for _ in 0..DRAWS {
        let bs = r.pick(&huge);
        let ch = r.range(1, 8);
        let bd = r.pick(TYPICAL_BITDEPTHS);
        f.assert_eq(bs, ch, bd);
    }
}

// ------------------------------------------------------------- C23 .. C24
// The `(sum + 7) / 8` truncating division: every residue class, both branches.

#[test]
fn c23_division_residues_mono() {
    let f = impls();
    for bs in 0..=64u32 {
        f.assert_eq(bs, 1, 1);
        f.assert_eq(bs, 1, 3);
        f.assert_eq(bs, 1, 8);
    }
}

#[test]
fn c24_division_residues_stereo() {
    let f = impls();
    for bs in 0..=64u32 {
        f.assert_eq(bs, 2, 1);
        f.assert_eq(bs, 2, 3);
        f.assert_eq(bs, 2, 32);
    }
    // Sweep the divisor residues right at the u32 wrap boundary too.
    for delta in 0..=16u32 {
        f.assert_eq(MAX - delta, 2, 1);
        f.assert_eq(MAX - delta, 1, 1);
    }
}

// ------------------------------------------------------------- C25 .. C28
// Wide sweeps.

#[test]
fn c25_global_random_fuzz_full_u32() {
    let f = impls();
    let mut r = rng(25);
    for _ in 0..2_000_000 {
        f.assert_eq(r.next_u32(), r.next_u32(), r.next_u32());
    }
}

#[test]
fn c26_interesting_value_cross_product() {
    let f = impls();
    let mut vals: Vec<u32> = vec![0, 1, 2, 3, 7, 8, 9, 17, 18, 31, 32, 33, MAX, MAX - 1, MAX - 17];
    for k in 0..32u32 {
        let p = 1u32 << k;
        vals.push(p);
        vals.push(p.wrapping_sub(1));
        vals.push(p.wrapping_add(1));
    }
    vals.sort_unstable();
    vals.dedup();
    for &bs in &vals {
        for &ch in &vals {
            for &bd in &vals {
                f.assert_eq(bs, ch, bd);
            }
        }
    }
}

#[test]
fn c27_exhaustive_small_cube() {
    let f = impls();
    for bs in 0..=40u32 {
        for ch in 0..=40u32 {
            for bd in 0..=40u32 {
                f.assert_eq(bs, ch, bd);
            }
        }
    }
}

#[test]
fn c28_realistic_flac_matrix() {
    let f = impls();
    for &bs in TYPICAL_BLOCKSIZES {
        for ch in 1..=8u32 {
            for &bd in TYPICAL_BITDEPTHS {
                f.assert_eq(bs, ch, bd);
            }
        }
    }
}

// ------------------------------------------------------------- C29 .. C30

#[test]
fn c29_stateless_interleaved_calls() {
    let f = impls();
    // Same arguments repeatedly, interleaving the two libraries: neither
    // implementation may carry hidden state or depend on call order.
    let first_c = f.c(4096, 2, 16);
    let first_r = f.rust(4096, 2, 16);
    assert_eq!(first_c, first_r);
    for i in 0..1000u32 {
        // perturb with other calls in between
        f.assert_eq(i, i % 5, i % 40);
        assert_eq!(f.c(4096, 2, 16), first_c, "C became stateful at iter {i}");
        assert_eq!(f.rust(4096, 2, 16), first_r, "Rust became stateful at iter {i}");
        assert_eq!(f.rust(4096, 2, 16), f.c(4096, 2, 16));
    }
}

#[test]
fn c30_abi_high_bit_arguments() {
    let f = impls();
    // High bit set in each argument position independently and together:
    // catches any sign-extension / i32-vs-u32 mismatch across the FFI boundary.
    let hi = [0x8000_0000u32, 0xFFFF_FFFF, 0x8000_0001, 0xDEAD_BEEF, 0xCAFE_BABE];
    for &a in &hi {
        for &b in &hi {
            for &c in &hi {
                f.assert_eq(a, b, c);
            }
        }
        // one high-bit arg at a time, others benign
        f.assert_eq(a, 2, 16);
        f.assert_eq(4096, a, 16);
        f.assert_eq(4096, 2, a);
        f.assert_eq(a, 1, 16);
        f.assert_eq(4096, 1, a);
    }
}
