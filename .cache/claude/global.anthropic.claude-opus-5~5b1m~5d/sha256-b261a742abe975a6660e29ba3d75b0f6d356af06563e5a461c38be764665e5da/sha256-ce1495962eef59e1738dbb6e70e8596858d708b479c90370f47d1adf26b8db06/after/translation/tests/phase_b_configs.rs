//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded from their `.so` via `libloading` and called
//! only through the exported `merge_sort` symbol. Every row asserts that the C
//! and Rust outputs are byte-identical in BOTH the `a` buffer (where the sorted
//! result lands) and the `b` scratch buffer.

mod common;

use common::*;

// --- Rows 1-11: the `size` axis (drives `hi-lo<=1` and the split shape) -----

#[test]
fn row01_size0() {
    run_row("01", &[0], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row02_size1() {
    run_row("02", &[1], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row03_size2_random() {
    run_row("03", &[2], K::Rand, T::Rand, P::Zero, F::Zero);
}

#[test]
fn row04_size2_tie_anti_texture() {
    // Minimal input that would expose the DEAD line-9 `texture_id` test.
    run_row("04", &[2], K::Eq, T::Anti, P::Zero, F::Zero);
}

#[test]
fn row05_size3_odd_split() {
    run_row("05", &[3], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row06_size4_balanced() {
    run_row("06", &[4], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row07_small_odd_sizes() {
    run_row("07", &[5, 7, 9], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row08_powers_of_two() {
    run_row("08", &[8, 16, 32, 256], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row09_powers_of_two_plus_minus_one() {
    run_row("09", &[15, 17, 31, 33, 255, 257], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row10_bulk_mixed_parity() {
    run_row("10", &[100, 1000], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row11_large_deep_recursion() {
    run_row("11", &[4096, 4097], K::Rand, T::Rand, P::Zero, F::Sentinel);
}

// --- Rows 12-22: the `sort_bits` / `texture_id` value axes ------------------

#[test]
fn row12_all_equal_keys() {
    run_row("12", &ALL_SIZES, K::Eq, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row13_all_equal_keys_anti_texture() {
    // The whole array ties on `sort_bits` while `texture_id` runs strictly
    // descending. If the C's line-9 branch were live the output would be ordered
    // by `texture_id`; because it is dead the output must stay in input order.
    run_row("13", &ALL_SIZES, K::Eq, T::Anti, P::Zero, F::Sentinel);
}

#[test]
fn row14_ascending() {
    run_row("14", &ALL_SIZES, K::Asc, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row15_descending() {
    run_row("15", &ALL_SIZES, K::Desc, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row16_few_distinct_keys() {
    run_row("16", &ALL_SIZES, K::Few, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row17_alternating() {
    run_row("17", &ALL_SIZES, K::Alt, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row18_all_negative_keys() {
    run_row("18", &ALL_SIZES, K::Neg, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row19_extreme_keys_and_textures() {
    run_row("19", &ALL_SIZES, K::Ext, T::Ext, P::Zero, F::Sentinel);
}

#[test]
fn row20_nearly_sorted() {
    run_row("20", &ALL_SIZES, K::One, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row21_sorted_with_duplicate_runs() {
    run_row("21", &ALL_SIZES, K::SortedDups, T::Rand, P::Zero, F::Sentinel);
}

#[test]
fn row22_constant_texture_id() {
    run_row("22", &ALL_SIZES, K::Rand, T::Zero, P::Zero, F::Sentinel);
}

// --- Rows 23-26: struct padding and scratch pre-fill -----------------------

#[test]
fn row23_padding_garbage_random_keys() {
    run_row("23", &ALL_SIZES, K::Rand, T::Rand, P::Garbage, F::Sentinel);
}

#[test]
fn row24_padding_garbage_all_ties() {
    run_row("24", &ALL_SIZES, K::Eq, T::Rand, P::Garbage, F::Sentinel);
}

#[test]
fn row25_padding_garbage_descending() {
    run_row("25", &ALL_SIZES, K::Desc, T::Rand, P::Garbage, F::Zero);
}

#[test]
fn row26_zeroed_scratch() {
    run_row("26", &ALL_SIZES, K::Rand, T::Rand, P::Zero, F::Zero);
}

// --- Row 27: aliased buffers (a == b) --------------------------------------

#[test]
fn row27_aliased_buffers() {
    let mut rng = Rng::new(SEED() ^ hash_str("27"));
    for size in [0i32, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 100] {
        let n = size as usize;
        for trial in 0..trials() {
            let a = gen_input(K::Rand, T::Rand, P::Garbage, n, &mut rng);
            let ctx = format!("row 27 [size={size} trial={trial}]");
            assert_same_aliased(&ctx, &a, size);
        }
    }
}

// --- Row 28: repeated invocation (no hidden state) -------------------------

#[test]
fn row28_called_twice() {
    let mut rng = Rng::new(SEED() ^ hash_str("28"));
    for &size in ALL_SIZES.iter() {
        let n = size.max(0) as usize;
        for trial in 0..trials() {
            let a0 = gen_input(K::Rand, T::Rand, P::Zero, n, &mut rng);
            let b0 = gen_scratch(F::Sentinel, n);

            // First call on both, then feed each implementation's own output
            // back into itself and compare again.
            let c1 = run_one(pair().c, &a0, &b0, size);
            let r1 = run_one(pair().rust, &a0, &b0, size);
            assert!(
                bytes(&c1.a) == bytes(&r1.a) && bytes(&c1.b) == bytes(&r1.b),
                "row 28 first call diverged [size={size} trial={trial}]"
            );

            let c2 = run_one(pair().c, &c1.a, &c1.b, size);
            let r2 = run_one(pair().rust, &r1.a, &r1.b, size);
            assert!(
                bytes(&c2.a) == bytes(&r2.a) && bytes(&c2.b) == bytes(&r2.b),
                "row 28 second call diverged [size={size} trial={trial}]"
            );
        }
    }
}

// --- Row 29: property-style fuzz over the full axis cross-product ----------

#[test]
fn row29_fuzz_full_cross_product() {
    let mut rng = Rng::new(SEED() ^ hash_str("29"));
    let iters: usize = std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    for it in 0..iters {
        let size = 1 + rng.below(600) as i32;
        let k = ALL_K[rng.below(ALL_K.len())];
        let t = ALL_T[rng.below(ALL_T.len())];
        let p = ALL_P[rng.below(ALL_P.len())];
        let f = ALL_F[rng.below(ALL_F.len())];
        let a = gen_input(k, t, p, size as usize, &mut rng);
        let b = gen_scratch(f, size as usize);
        let ctx = format!("row 29 fuzz [it={it} size={size} K={k:?} T={t:?} P={p:?} F={f:?}]");
        assert_same(&ctx, &a, &b, size);
    }
}

// --- Row 30: scratch buffer longer than `size` (slack tail) ----------------

#[test]
fn row30_oversized_scratch_buffer() {
    let mut rng = Rng::new(SEED() ^ hash_str("30"));
    for &size in ALL_SIZES.iter() {
        let n = size.max(0) as usize;
        for trial in 0..trials() {
            let slack = 1 + rng.below(8);
            // `a` also gets slack so an over-read/over-write of either buffer
            // past `size` shows up as a difference instead of a crash.
            let mut a = gen_input(K::Rand, T::Rand, P::Garbage, n + slack, &mut rng);
            // Mark the tail of `a` distinctly.
            for (i, e) in a[n..].iter_mut().enumerate() {
                e.texture_id = 0xDEAD_0000_0000_0000 | i as u64;
                e.sort_bits = i32::MIN + i as i32;
                e.pad = 0x5555_5555;
            }
            let b = gen_scratch(F::Sentinel, n + slack);
            let ctx = format!("row 30 [size={size} slack={slack} trial={trial}]");
            assert_same(&ctx, &a, &b, size);
        }
    }
}
