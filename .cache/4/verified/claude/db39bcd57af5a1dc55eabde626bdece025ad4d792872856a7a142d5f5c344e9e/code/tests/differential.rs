//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every test calls BOTH the C `.so` and
//! the Rust `.so` through `libloading` and asserts the returned `u32`s are
//! byte-identical (plus a third-opinion oracle check).

mod common;

use common::*;

/// Iterations for the ordinary randomized rows.
const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Row 1-7: the stereo path (channels == 2), crossed with the bitdepth axis
// ---------------------------------------------------------------------------

#[test]
fn row01_stereo_bitdepth_exactly_32() {
    let l = libs();
    let mut rng = Rng::new(1);
    for _ in 0..N {
        let bs = rng.range(1, 65535);
        assert_same_triple(l, bs, 2, 32);
    }
}

#[test]
fn row02_stereo_bitdepth_16() {
    let l = libs();
    let mut rng = Rng::new(2);
    for _ in 0..N {
        let bs = rng.range(1, 65535);
        assert_same_triple(l, bs, 2, 16);
    }
}

#[test]
fn row03_stereo_bitdepth_1_to_31() {
    let l = libs();
    let mut rng = Rng::new(3);
    for _ in 0..N {
        let bs = rng.range(1, 65535);
        let bd = rng.range(1, 31);
        assert_same_triple(l, bs, 2, bd);
    }
}

#[test]
fn row04_stereo_bitdepth_33_to_64() {
    let l = libs();
    let mut rng = Rng::new(4);
    for _ in 0..N {
        let bs = rng.range(1, 65535);
        let bd = rng.range(33, 64);
        assert_same_triple(l, bs, 2, bd);
    }
}

#[test]
fn row05_stereo_bitdepth_zero_full_range_blocksize() {
    let l = libs();
    let mut rng = Rng::new(5);
    for _ in 0..N {
        let bs = rng.next_u32();
        // T3 collapses to blocksize * (0 + 1) * 1 == blocksize.
        assert_same_triple(l, bs, 2, 0);
    }
}

#[test]
fn row06_stereo_bitdepth_uint32_max_inner_wrap() {
    let l = libs();
    let mut rng = Rng::new(6);
    for _ in 0..N {
        let bs = rng.next_u32();
        // bitdepth + (bitdepth != 32) == 0xFFFFFFFF + 1 == 0 (wraps).
        assert_same_triple(l, bs, 2, u32::MAX);
    }
}

#[test]
fn row07_stereo_full_range_blocksize_and_bitdepth() {
    let l = libs();
    let mut rng = Rng::new(7);
    for _ in 0..N {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, 2, bd);
    }
}

// ---------------------------------------------------------------------------
// Row 8-14: the non-stereo path (channels != 2)
// ---------------------------------------------------------------------------

#[test]
fn row08_channels_zero_all_terms_vanish() {
    let l = libs();
    let mut rng = Rng::new(8);
    for _ in 0..N {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        // channels * (channels != 2) == 0, so the result is exactly 18.
        assert_same_and_eq(l, bs, 0, bd, 18);
    }
}

#[test]
fn row09_mono_realistic() {
    let l = libs();
    let mut rng = Rng::new(9);
    for _ in 0..N {
        let bs = rng.pick(FLAC_BLOCKSIZES);
        let bd = rng.pick(FLAC_BITDEPTHS);
        assert_same_triple(l, bs, 1, bd);
    }
}

#[test]
fn row10_channels_three_just_past_stereo() {
    let l = libs();
    let mut rng = Rng::new(10);
    for _ in 0..N {
        let bs = rng.range(1, 65535);
        let bd = rng.range(1, 32);
        assert_same_triple(l, bs, 3, bd);
    }
}

#[test]
fn row11_channels_4_to_8_realistic() {
    let l = libs();
    let mut rng = Rng::new(11);
    for _ in 0..N {
        let ch = rng.range(4, 8);
        let bs = rng.pick(FLAC_BLOCKSIZES);
        let bd = rng.pick(FLAC_BITDEPTHS);
        assert_same_triple(l, bs, ch, bd);
    }
}

#[test]
fn row12_channels_9_to_255() {
    let l = libs();
    let mut rng = Rng::new(12);
    for _ in 0..N {
        let ch = rng.range(9, 255);
        let bs = rng.range(1, 8192);
        let bd = rng.range(1, 64);
        assert_same_triple(l, bs, ch, bd);
    }
}

#[test]
fn row13_huge_channels_forces_t1_and_outer_wrap() {
    let l = libs();
    let mut rng = Rng::new(13);
    for _ in 0..N {
        let mut ch = rng.range(1 << 16, u32::MAX);
        if ch == 2 {
            ch = 3; // keep this row on the non-stereo path
        }
        let bs = rng.range(1, u32::MAX);
        let bd = rng.range(1, u32::MAX);
        assert_same_triple(l, bs, ch, bd);
    }
}

#[test]
fn row14_channels_uint32_max_outer_wrap() {
    let l = libs();
    let mut rng = Rng::new(14);
    for _ in 0..N {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, u32::MAX, bd);
    }
}

// ---------------------------------------------------------------------------
// Row 15-18: the blocksize magnitude axis
// ---------------------------------------------------------------------------

#[test]
fn row15_blocksize_zero() {
    let l = libs();
    let mut rng = Rng::new(15);
    for _ in 0..N {
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        // Every term is multiplied by blocksize == 0, so (0 + 7) / 8 == 0.
        assert_same_and_eq(l, 0, ch, bd, 18u32.wrapping_add(ch));
    }
}

#[test]
fn row16_blocksize_one() {
    let l = libs();
    let mut rng = Rng::new(16);
    for _ in 0..N {
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, 1, ch, bd);
    }
}

#[test]
fn row17_blocksize_uint32_max() {
    let l = libs();
    let mut rng = Rng::new(17);
    for _ in 0..N {
        let ch = rng.range(0, 8);
        let bd = rng.range(0, 64);
        assert_same_triple(l, u32::MAX, ch, bd);
    }
}

#[test]
fn row18_blocksize_powers_of_two_exhaustive() {
    let l = libs();
    for shift in 0..32u32 {
        let bs = 1u32 << shift;
        for ch in [1u32, 2, 3] {
            for bd in [16u32, 24, 32] {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19-21: the ceiling-divide (+7)/8 rounding axis and its wrap
// ---------------------------------------------------------------------------

#[test]
fn row19_rounding_residues_non_stereo() {
    let l = libs();
    // channels == 1, bitdepth == 1  =>  T1 == blocksize, so numerator == bs + 7
    // and `bs % 8` walks every residue class.
    for residue in 0..8u32 {
        for k in 0..2048u32 {
            let bs = k * 8 + residue;
            assert_same_triple(l, bs, 1, 1);
        }
    }
    // Also sweep every residue at large magnitudes.
    let mut rng = Rng::new(19);
    for _ in 0..N {
        let bs = rng.next_u32();
        assert_same_triple(l, bs, 1, 1);
    }
}

#[test]
fn row20_rounding_residues_stereo() {
    let l = libs();
    // channels == 2, bitdepth == 1 => T2 == bs, T3 == bs * 2, sum == 3 * bs.
    for residue in 0..8u32 {
        for k in 0..2048u32 {
            let bs = k * 8 + residue;
            assert_same_triple(l, bs, 2, 1);
        }
    }
    let mut rng = Rng::new(20);
    for _ in 0..N {
        let bs = rng.next_u32();
        assert_same_triple(l, bs, 2, 1);
    }
}

#[test]
fn row21_numerator_plus_seven_wraps() {
    let l = libs();

    // channels == 1, bitdepth == 1 => numerator == blocksize + 7, so the +7
    // itself overflows for blocksize in [UINT32_MAX-6, UINT32_MAX].
    for bs in (u32::MAX - 6)..=u32::MAX {
        assert_same_triple(l, bs, 1, 1);
    }

    // Same wrap reached on the stereo path: bitdepth == UINT32_MAX makes
    // T2 == blocksize * 0xFFFFFFFF and T3 == 0.
    for bs in 1..=64u32 {
        assert_same_triple(l, bs, 2, u32::MAX);
    }

    // CONSTRUCTIVE search. Randomly sampling for `sum >= UINT32_MAX - 6` is
    // hopeless (probability ~1.6e-9, so a 400k-iteration loop finds zero cases
    // and asserts nothing). Instead we SOLVE for the blocksize that produces a
    // chosen numerator: for a non-stereo `channels` and odd multiplier
    // `M = bitdepth * channels`, `M` is invertible mod 2^32, so
    // `blocksize = target * M^-1` gives `blocksize * M == target` exactly.
    let mut rng = Rng::new(21);
    let mut wrap_cases = 0usize;
    for _ in 0..20_000 {
        // Odd bitdepth and odd channels (channels != 2) => M is odd => invertible.
        let bd = rng.range(0, 1 << 15) * 2 + 1;
        let mut ch = rng.range(0, 1 << 15) * 2 + 1;
        if ch == 2 {
            ch = 3;
        }
        let m = bd.wrapping_mul(ch);
        debug_assert_eq!(m & 1, 1, "M must be odd to be invertible mod 2^32");
        let inv = inverse_mod_2_32(m);

        // Every numerator in the wrap window, plus the two just below it.
        for target in [
            u32::MAX,
            u32::MAX - 1,
            u32::MAX - 2,
            u32::MAX - 3,
            u32::MAX - 4,
            u32::MAX - 5,
            u32::MAX - 6,
            u32::MAX - 7,
            u32::MAX - 8,
        ] {
            let bs = target.wrapping_mul(inv);
            // Confirm we really hit the intended sum before asserting on it.
            assert_eq!(
                term_sum(bs, ch, bd),
                target,
                "constructive solve failed for M={m} target={target}"
            );
            if target >= u32::MAX - 6 {
                wrap_cases += 1;
            }
            assert_same_triple(l, bs, ch, bd);
        }
    }
    assert!(
        wrap_cases >= 20_000,
        "expected many constructed numerator-wrap cases, got {wrap_cases}"
    );
    println!("row21: {wrap_cases} constructed numerator-wrap cases verified");
}

/// Multiplicative inverse of an odd `u32` modulo 2^32 (Newton iteration).
fn inverse_mod_2_32(a: u32) -> u32 {
    assert_eq!(a & 1, 1, "only odd values are invertible mod 2^32");
    let mut x = a; // correct to 3 bits
    for _ in 0..4 {
        // doubles the number of correct bits each step: 3 -> 6 -> 12 -> 24 -> 48
        x = x.wrapping_mul(2u32.wrapping_sub(a.wrapping_mul(x)));
    }
    debug_assert_eq!(a.wrapping_mul(x), 1);
    x
}

/// The three C terms summed WITHOUT the trailing `+7`, used only to steer the
/// search in `row21`.
fn term_sum(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let is_stereo = u32::from(channels == 2);
    let not_stereo = u32::from(channels != 2);
    let not_32 = u32::from(bitdepth != 32);
    let t1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(not_stereo));
    let t2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(is_stereo);
    let t3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(not_32))
        .wrapping_mul(is_stereo);
    t1.wrapping_add(t2).wrapping_add(t3)
}

// ---------------------------------------------------------------------------
// Row 22-24: exhaustive sweeps
// ---------------------------------------------------------------------------

#[test]
fn row22_exhaustive_small_cube() {
    let l = libs();
    let mut count = 0usize;
    for bs in 0..=48u32 {
        for ch in 0..=8u32 {
            for bd in 0..=40u32 {
                assert_same_triple(l, bs, ch, bd);
                count += 1;
            }
        }
    }
    assert_eq!(count, 49 * 9 * 41);
}

#[test]
fn row23_exhaustive_single_axis_sweeps() {
    let l = libs();

    // channels 0..=1024 at several realistic (blocksize, bitdepth) pairs.
    for ch in 0..=1024u32 {
        for (bs, bd) in [(4096u32, 16u32), (1152, 24), (256, 32), (65535, 8)] {
            assert_same_triple(l, bs, ch, bd);
        }
    }

    // bitdepth 0..=64 crossed with the stereo predicate.
    for bd in 0..=64u32 {
        for ch in [0u32, 1, 2, 3, 8] {
            assert_same_triple(l, 4096, ch, bd);
        }
    }

    // blocksize 0..=4096 crossed with the stereo predicate and the 32 predicate.
    for bs in 0..=4096u32 {
        for ch in [1u32, 2, 3] {
            for bd in [0u32, 16, 32, 33] {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
}

#[test]
fn row24_exhaustive_flac_realistic_matrix() {
    let l = libs();
    for &bs in FLAC_BLOCKSIZES {
        for ch in 1..=8u32 {
            for &bd in FLAC_BITDEPTHS {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25-26: unconstrained fuzz and boundary interaction
// ---------------------------------------------------------------------------

#[test]
fn row25_uniform_random_full_range() {
    let l = libs();
    let mut rng = Rng::new(25);
    for _ in 0..200_000 {
        let bs = rng.next_u32();
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, ch, bd);
    }
}

#[test]
fn row26_boundary_interaction_exhaustive_plus_random() {
    let l = libs();

    // Full cross product of the boundary set on all three arguments.
    for &bs in BOUNDARY_VALUES {
        for &ch in BOUNDARY_VALUES {
            for &bd in BOUNDARY_VALUES {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }

    // Mixed: each argument independently either a boundary value or uniform
    // random, so boundary/random interactions are covered too.
    let mut rng = Rng::new(26);
    for _ in 0..N {
        let pick = |r: &mut Rng| {
            if r.next_u64() & 1 == 0 {
                r.pick(BOUNDARY_VALUES)
            } else {
                r.next_u32()
            }
        };
        let bs = pick(&mut rng);
        let ch = pick(&mut rng);
        let bd = pick(&mut rng);
        assert_same_triple(l, bs, ch, bd);
    }
}

// ---------------------------------------------------------------------------
// Sanity: the harness really is loading two distinct shared objects.
// ---------------------------------------------------------------------------

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let l = libs();
    println!("C   .so: {}", l.c_path.display());
    println!("Rust.so: {}", l.rust_path.display());
    assert_ne!(
        l.c_path, l.rust_path,
        "the two libraries must be different files"
    );
    assert!(l.c_path.is_file() && l.rust_path.is_file());
    // Known-good spot check confirmed against the compiled C library.
    assert_same_and_eq(l, 0, 0, 0, 18);
    assert_same_and_eq(l, 4096, 2, 16, 16916);
    assert_same_and_eq(l, 4096, 1, 16, 8211);
    assert_same_and_eq(l, 4096, 2, 32, 32788);
}
