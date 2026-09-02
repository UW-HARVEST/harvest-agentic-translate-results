//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row runs many randomized inputs from
//! a fixed seed and compares the full byte image of BOTH output buffers.

mod common;

use common::*;

// --- row 1 -----------------------------------------------------------------
#[test]
fn row01_size_zero_touches_nothing() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..64 {
        // Non-empty allocations so a divergent implementation that writes
        // anything at all is caught, but size == 0 passed to the API.
        let a = gen_full_random(&mut rng, 8);
        let b = garbage_scratch(&mut rng, 8);
        diff_with_size_on(&pair, "row01 size=0 non-empty buffers", &a, &b, 0);
    }
    // And the genuinely empty / null-pointer case.
    diff_with_size_on(&pair, "row01 size=0 null pointers", &[], &[], 0);
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn row02_size_one() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..256 {
        let a = gen_full_random(&mut rng, 1);
        let b = garbage_scratch(&mut rng, 1);
        diff_on(&pair, "row02 size=1", &a, &b);
    }
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn row03_size_two_all_orderings() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..128 {
        let t0 = rng.next_u64();
        let t1 = rng.next_u64();
        let p0 = rng.bytes4();
        let p1 = rng.bytes4();
        let x = rng.next_i32();
        let y = rng.next_i32();
        let cases: [[i32; 2]; 6] = [
            [x, y],
            [y, x],
            [x, x],
            [i32::MIN, i32::MAX],
            [i32::MAX, i32::MIN],
            [0, 0],
        ];
        for [b0, b1] in cases {
            let a = vec![Sprite::new(t0, b0, p0), Sprite::new(t1, b1, p1)];
            let b = garbage_scratch(&mut rng, 2);
            diff_on(&pair, "row03 size=2", &a, &b);
        }
    }
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn row04_size_three_odd_split() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..512 {
        let a: Vec<Sprite> = (0..3)
            .map(|_| Sprite::new(rng.next_u64(), rng.below(3) as i32, rng.bytes4()))
            .collect();
        let b = garbage_scratch(&mut rng, 3);
        diff_on(&pair, "row04 size=3", &a, &b);
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn row05_power_of_two_sizes() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 5);
    for &n in POW2_SIZES {
        for _ in 0..REPS {
            let a = gen_full_random(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row05 pow2 n={n}"), &a, &b);
        }
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn row06_non_power_of_two_sizes() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 6);
    for &n in NON_POW2_SIZES {
        for _ in 0..REPS {
            let a = gen_full_random(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row06 nonpow2 n={n}"), &a, &b);
        }
    }
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn row07_all_sort_bits_equal_distinct_textures() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 7);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_all_bits_equal(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row07 equal-bits n={n}"), &a, &b);
        }
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn row08_total_duplicates() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 8);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_total_duplicates(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row08 all-dup n={n}"), &a, &b);
        }
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn row09_already_ascending() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 9);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_ascending(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row09 ascending n={n}"), &a, &b);
        }
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn row10_already_descending() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 10);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_descending(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row10 descending n={n}"), &a, &b);
        }
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn row11_two_valued_sort_bits() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 11);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_two_valued(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row11 two-valued n={n}"), &a, &b);
        }
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn row12_small_range_sort_bits() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 12);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_small_range(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row12 small-range n={n}"), &a, &b);
        }
    }
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn row13_extreme_signed_sort_bits() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 13);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_extreme_bits(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row13 extreme-bits n={n}"), &a, &b);
        }
    }
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn row14_only_int_min_and_int_max() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 14);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_minmax_bits(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row14 minmax-bits n={n}"), &a, &b);
        }
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn row15_extreme_texture_ids() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 15);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_extreme_texture(&mut rng, n);
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row15 extreme-tex n={n}"), &a, &b);
        }
    }
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn row16_garbage_padding_is_propagated() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 16);
    for &n in SIZES {
        for _ in 0..REPS {
            // Every padding byte non-zero so a 12-byte copy would show up.
            let a: Vec<Sprite> = (0..n)
                .map(|_| {
                    let mut p = rng.bytes4();
                    for byte in p.iter_mut() {
                        if *byte == 0 {
                            *byte = 0xA5;
                        }
                    }
                    Sprite::new(rng.next_u64(), rng.below(3) as i32, p)
                })
                .collect();
            let b = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row16 garbage-padding n={n}"), &a, &b);
        }
    }
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn row17_scratch_buffer_final_state() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 17);
    for &n in SIZES {
        for _ in 0..REPS {
            let a = gen_full_random(&mut rng, n);
            // Zeroed scratch and garbage scratch: the sorted result lands in
            // `a` or in `b` depending on the recursion-depth parity, so both
            // buffers are compared by `diff_on`.
            diff_on(&pair, &format!("row17 zero-scratch n={n}"), &a, &zero_scratch(n));
            let g = garbage_scratch(&mut rng, n);
            diff_on(&pair, &format!("row17 garbage-scratch n={n}"), &a, &g);
        }
    }
}

// --- row 18 ----------------------------------------------------------------
#[test]
fn row18_all_axes_at_once_deep_recursion() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..4 {
        let n = 4096;
        let bits = rng.next_i32();
        let a: Vec<Sprite> = (0..n)
            .map(|_| Sprite::new(rng.next_u64(), bits, rng.bytes4()))
            .collect();
        let b = garbage_scratch(&mut rng, n);
        diff_on(&pair, "row18 n=4096 equal-bits garbage-everything", &a, &b);
    }
}

// --- row 19 ----------------------------------------------------------------
#[test]
fn row19_fuzz_sweep() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..400 {
        let n = rng.below(257) as usize;
        let a = gen_full_random(&mut rng, n);
        let b = garbage_scratch(&mut rng, n);
        diff_on(&pair, &format!("row19 fuzz #{i} n={n}"), &a, &b);
    }
}

// --- row 20 ----------------------------------------------------------------
#[test]
fn row20_repeated_invocation() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 20);
    for &n in SIZES {
        for _ in 0..REPS {
            let a0 = gen_full_random(&mut rng, n);
            let b0 = garbage_scratch(&mut rng, n);

            let mut a_c = a0.clone();
            let mut b_c = b0.clone();
            let mut a_r = a0.clone();
            let mut b_r = b0.clone();

            let (ac, bc) = if n == 0 {
                (std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                (a_c.as_mut_ptr(), b_c.as_mut_ptr())
            };
            let (ar, br) = if n == 0 {
                (std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                (a_r.as_mut_ptr(), b_r.as_mut_ptr())
            };

            for pass in 0..3 {
                unsafe { (pair.c)(ac, bc, n as i32) };
                unsafe { (pair.rust)(ar, br, n as i32) };
                assert!(
                    a_c == a_r && b_c == b_r,
                    "DIVERGENCE [row20 repeated n={n} pass={pass}]\n\
                     C a={a_c:?}\nR a={a_r:?}\nC b={b_c:?}\nR b={b_r:?}"
                );
            }
        }
    }
}

// --- lower-level branch coverage -------------------------------------------
//
// The three `static` helpers are not exported (see SYMBOLS.md), so they are
// driven through `merge_sort`. These cases target the specific branches listed
// in the CONFIGS.md branch table.

#[test]
fn lowlevel_branch_matrix() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xB0);

    // `iteration`: right run exhausted first (`j >= hi` short-circuit) — the
    // second half is entirely smaller than the first half.
    for &n in SIZES {
        if n < 2 {
            continue;
        }
        let half = n / 2;
        let a: Vec<Sprite> = (0..n)
            .map(|i| {
                let bits = if i < half { 1000 } else { -1000 };
                Sprite::new(rng.next_u64(), bits, rng.bytes4())
            })
            .collect();
        diff_on(&pair, &format!("lowlevel right-exhausted n={n}"), &a, &garbage_scratch(&mut rng, n));

        // `iteration`: left run exhausted first (`i >= split`).
        let a: Vec<Sprite> = (0..n)
            .map(|i| {
                let bits = if i < half { -1000 } else { 1000 };
                Sprite::new(rng.next_u64(), bits, rng.bytes4())
            })
            .collect();
        diff_on(&pair, &format!("lowlevel left-exhausted n={n}"), &a, &garbage_scratch(&mut rng, n));

        // Perfect interleave: comparator alternates every step.
        let a: Vec<Sprite> = (0..n)
            .map(|i| Sprite::new(rng.next_u64(), (i % 2) as i32 * 7, rng.bytes4()))
            .collect();
        diff_on(&pair, &format!("lowlevel interleave n={n}"), &a, &garbage_scratch(&mut rng, n));
    }
}

/// The `texture_id` tiebreak in `spritebatch_internal_sprite_less_than_or_equal`
/// is dead code (ERRORS.md rows 1–2). Both implementations must therefore leave
/// `texture_id` order alone within a run of equal `sort_bits`. Verified
/// differentially *and* asserted against the C's actual observed behaviour so a
/// "fixed" Rust comparator cannot pass.
#[test]
fn dead_texture_id_tiebreak_matches_c() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xDE);
    for &n in &[2usize, 3, 4, 5, 8, 16, 33, 64, 129] {
        for _ in 0..32 {
            // All sort_bits equal, texture_ids strictly descending.
            let a: Vec<Sprite> = (0..n)
                .map(|i| Sprite::new((n - i) as u64 * 1000 + rng.below(7), 42, [0; 4]))
                .collect();
            let b = zero_scratch(n);
            diff_on(&pair, &format!("dead-tiebreak n={n}"), &a, &b);
        }
    }
}
