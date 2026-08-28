//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md`. Every row drives the exported `normalize` symbol of BOTH the
//! C `.so` and the Rust `.so` over many seeded-random inputs and compares every
//! byte of every touched (and untouched) region.

mod common;

use common::*;
use std::ffi::c_int;

#[derive(Clone, Copy)]
enum Shape {
    Disjoint,
    InPlace,
    Fwd(isize),
    Bwd(isize),
    FwdRand,
    BwdRand,
}

/// Drive one `CONFIGS.md` row: cross `sizes` x `offsets` x `trials` random
/// inputs from `dist`, in the given aliasing `shape`.
fn run_row(seed: u64, dist: Dist, offsets: &[usize], shape: Shape, sizes: &[i32], trials: usize) {
    let mut rng = Rng::new(seed);
    let mut cases = 0usize;
    for &sz in sizes {
        let n = sz.max(0) as usize;
        for &off in offsets {
            for _ in 0..trials {
                let data = gen_data(dist, n, &mut rng);
                let s = match shape {
                    Shape::Disjoint => Scenario::disjoint(&data, off, sz),
                    Shape::InPlace => Scenario::in_place(&data, off, sz),
                    Shape::Fwd(k) => {
                        if n < 2 {
                            continue;
                        }
                        Scenario::overlap(&data, k, sz)
                    }
                    Shape::Bwd(k) => {
                        if n < 2 {
                            continue;
                        }
                        Scenario::overlap(&data, k, sz)
                    }
                    Shape::FwdRand => {
                        if n < 2 {
                            continue;
                        }
                        let k = 1 + rng.below(n - 1);
                        Scenario::overlap(&data, k as isize, sz)
                    }
                    Shape::BwdRand => {
                        if n < 2 {
                            continue;
                        }
                        let k = 1 + rng.below(n - 1);
                        Scenario::overlap(&data, -(k as isize), sz)
                    }
                };
                assert_same(&s);
                cases += 1;
            }
        }
    }
    assert!(cases > 0, "row produced no cases");
}

const OFF0: &[usize] = &[0];
const OFF123: &[usize] = &[1, 2, 3];

// --- rows 1..14: disjoint buffers, every value distribution -----------------

#[test] // row 1
fn cfg_01_disjoint_unit_aligned() {
    run_row(0x0001, Dist::Unit, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 2
fn cfg_02_disjoint_unit_misaligned() {
    run_row(0x0002, Dist::Unit, OFF123, Shape::Disjoint, SIZES, 20);
}

#[test] // row 3
fn cfg_03_disjoint_wide_aligned() {
    run_row(0x0003, Dist::Wide, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 4
fn cfg_04_disjoint_wide_misaligned() {
    run_row(0x0004, Dist::Wide, OFF123, Shape::Disjoint, SIZES, 20);
}

#[test] // row 5
fn cfg_05_disjoint_finitebits_aligned() {
    run_row(0x0005, Dist::FiniteBits, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 6
fn cfg_06_disjoint_finitebits_misaligned() {
    run_row(0x0006, Dist::FiniteBits, OFF123, Shape::Disjoint, SIZES, 20);
}

#[test] // row 7
fn cfg_07_disjoint_pow2() {
    run_row(0x0007, Dist::Pow2, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 8
fn cfg_08_disjoint_dominant() {
    run_row(0x0008, Dist::Dominant, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 9
fn cfg_09_disjoint_all_equal() {
    run_row(0x0009, Dist::AllEqual, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 10
fn cfg_10_disjoint_subnormal() {
    run_row(0x000A, Dist::Subnormal, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 11
fn cfg_11_disjoint_one_hot() {
    run_row(0x000B, Dist::OneHot, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 12
fn cfg_12_disjoint_signed_zeros() {
    run_row(0x000C, Dist::SignedZeros, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 13
fn cfg_13_disjoint_sum_is_one() {
    run_row(0x000D, Dist::SumIsOne, OFF0, Shape::Disjoint, SIZES, 20);
}

#[test] // row 14
fn cfg_14_disjoint_tiny() {
    run_row(0x000E, Dist::Tiny, OFF0, Shape::Disjoint, SIZES, 40);
}

// --- rows 15..22: exact in-place (dest == src) ------------------------------

#[test] // row 15
fn cfg_15_in_place_unit_aligned() {
    run_row(0x000F, Dist::Unit, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 16
fn cfg_16_in_place_unit_misaligned() {
    run_row(0x0010, Dist::Unit, OFF123, Shape::InPlace, SIZES, 20);
}

#[test] // row 17
fn cfg_17_in_place_wide() {
    run_row(0x0011, Dist::Wide, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 18
fn cfg_18_in_place_finitebits() {
    run_row(0x0012, Dist::FiniteBits, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 19
fn cfg_19_in_place_pow2() {
    run_row(0x0013, Dist::Pow2, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 20
fn cfg_20_in_place_dominant() {
    run_row(0x0014, Dist::Dominant, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 21
fn cfg_21_in_place_subnormal() {
    run_row(0x0015, Dist::Subnormal, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 22
fn cfg_22_in_place_signed_zeros() {
    run_row(0x0016, Dist::SignedZeros, OFF0, Shape::InPlace, SIZES, 40);
}

// --- rows 23..29: partial overlap ------------------------------------------

#[test] // row 23
fn cfg_23_overlap_fwd_1_unit() {
    run_row(0x0017, Dist::Unit, OFF0, Shape::Fwd(1), SIZES, 40);
}

#[test] // row 24
fn cfg_24_overlap_fwd_rand_unit() {
    run_row(0x0018, Dist::Unit, OFF0, Shape::FwdRand, SIZES, 40);
}

#[test] // row 25
fn cfg_25_overlap_fwd_rand_wide() {
    run_row(0x0019, Dist::Wide, OFF0, Shape::FwdRand, SIZES, 40);
}

#[test] // row 26
fn cfg_26_overlap_fwd_rand_finitebits() {
    run_row(0x001A, Dist::FiniteBits, OFF0, Shape::FwdRand, SIZES, 40);
}

#[test] // row 27
fn cfg_27_overlap_bwd_1_unit() {
    run_row(0x001B, Dist::Unit, OFF0, Shape::Bwd(-1), SIZES, 40);
}

#[test] // row 28
fn cfg_28_overlap_bwd_rand_wide() {
    run_row(0x001C, Dist::Wide, OFF0, Shape::BwdRand, SIZES, 40);
}

#[test] // row 29
fn cfg_29_overlap_bwd_rand_finitebits() {
    run_row(0x001D, Dist::FiniteBits, OFF0, Shape::BwdRand, SIZES, 40);
}

// --- row 30: `size` shorter than the live window (partial write) ------------

#[test]
fn cfg_30_partial_write_leaves_tail_untouched() {
    let mut rng = Rng::new(0x001E);
    for &sz in SIZES {
        let n = sz as usize;
        for extra in [1usize, 2, 3, 5, 16] {
            for _ in 0..10 {
                // The live window is `n + extra` floats long but `size` is only
                // `n`, so the last `extra` floats of `dest` must stay sentinel.
                let data = gen_data(Dist::Unit, n + extra, &mut rng);
                assert_same(&Scenario::disjoint(&data, 0, sz));
                let data = gen_data(Dist::Wide, n + extra, &mut rng);
                assert_same(&Scenario::in_place(&data, 0, sz));
            }
        }
    }
}

// --- rows 31..33 -----------------------------------------------------------

#[test] // row 31
fn cfg_31_disjoint_small_integers_exact() {
    run_row(0x001F, Dist::SmallInts, OFF0, Shape::Disjoint, SIZES, 40);
}

#[test] // row 32
fn cfg_32_sum_overflows_only_for_large_size() {
    run_row(0x0020, Dist::OverflowEdge, OFF0, Shape::Disjoint, SIZES, 40);
    run_row(0x0021, Dist::OverflowEdge, OFF0, Shape::InPlace, SIZES, 40);
}

#[test] // row 33
fn cfg_33_in_place_sum_is_one_bit_identity() {
    run_row(0x0022, Dist::SumIsOne, OFF0, Shape::InPlace, SIZES, 20);
}

// --- rows 34..35: exhaustive small `size` sweep, no gaps --------------------

#[test] // row 34
fn cfg_34_exhaustive_small_sizes_disjoint() {
    let sizes: Vec<i32> = (0..=300).collect();
    run_row(0x0023, Dist::Unit, OFF0, Shape::Disjoint, &sizes, 3);
}

#[test] // row 35
fn cfg_35_exhaustive_small_sizes_in_place() {
    let sizes: Vec<i32> = (0..=300).collect();
    run_row(0x0024, Dist::Wide, OFF0, Shape::InPlace, &sizes, 3);
}

// --- row 36: long loops ----------------------------------------------------

#[test] // row 36
fn cfg_36_large_sizes() {
    let sizes: &[i32] = &[4096, 16384, 65536];
    run_row(0x0025, Dist::Unit, OFF0, Shape::Disjoint, sizes, 3);
    run_row(0x0026, Dist::Wide, OFF0, Shape::Disjoint, sizes, 3);
    run_row(0x0027, Dist::Unit, OFF0, Shape::InPlace, sizes, 3);
    run_row(0x0028, Dist::Wide, OFF0, Shape::FwdRand, sizes, 3);
}

// --- extra: a wide unseeded-shape smoke sweep over mixed everything --------

#[test]
fn cfg_zz_mixed_fuzz() {
    let dists = [
        Dist::Unit,
        Dist::Wide,
        Dist::FiniteBits,
        Dist::Pow2,
        Dist::Dominant,
        Dist::AllEqual,
        Dist::Subnormal,
        Dist::OneHot,
        Dist::SignedZeros,
        Dist::SumIsOne,
        Dist::Tiny,
        Dist::SmallInts,
        Dist::OverflowEdge,
    ];
    let mut rng = Rng::new(0xDEAD_BEEF);
    for iter in 0..4000u32 {
        let dist = dists[rng.below(dists.len())];
        let n = rng.below(70);
        let sz = n as c_int;
        let off = rng.below(4);
        let data = gen_data(dist, n, &mut rng);
        let s = match rng.below(4) {
            0 => Scenario::disjoint(&data, off, sz),
            1 => Scenario::in_place(&data, off, sz),
            2 if n >= 2 => Scenario::overlap(&data, (1 + rng.below(n - 1)) as isize, sz),
            3 if n >= 2 => Scenario::overlap(&data, -((1 + rng.below(n - 1)) as isize), sz),
            _ => Scenario::disjoint(&data, off, sz),
        };
        let mut s = s;
        s.label = format!("mixed_fuzz iter={iter} dist={dist:?} {}", s.label);
        assert_same(&s);
    }
}
