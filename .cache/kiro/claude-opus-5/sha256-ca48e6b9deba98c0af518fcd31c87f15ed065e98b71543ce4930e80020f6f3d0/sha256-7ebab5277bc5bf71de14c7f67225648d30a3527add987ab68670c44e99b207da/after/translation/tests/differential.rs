//! Differential tests: C reference `.so` vs Rust `.so`, both loaded with
//! `libloading` and called only through their exported `merge_sort` symbol.
//!
//! Ordered bottom-up over the C call hierarchy:
//!   1. `spritebatch_internal_sprite_less_than_or_equal` (via size-2 inputs)
//!   2. `spritebatch_internal_merge_sort_iteration`      (via size 2..=4)
//!   3. `spritebatch_internal_merge_sort_recurse`        (via exhaustive sizes)
//!   4. `merge_sort`                                     (public entry point)
//! The internal helpers are `static` in C, so they are reachable only through
//! `merge_sort`; the inputs below are chosen to drive each of their branches.

mod common;

use common::{Impls, Rng, Sprite};

// ---------------------------------------------------------------------------
// Level 0: the .so files themselves
// ---------------------------------------------------------------------------

#[test]
fn both_libraries_export_merge_sort() {
    let impls = Impls::load();
    // `load()` panics if either symbol is missing.
    assert!(impls.c_path.exists());
    assert!(impls.rust_path.exists());
}

// ---------------------------------------------------------------------------
// Level 1: comparator branches, exercised with size == 2
// ---------------------------------------------------------------------------

#[test]
fn comparator_branches_size_two() {
    let impls = Impls::load();

    // (sort_bits_a, tex_a, sort_bits_b, tex_b)
    let cases: &[(i32, u64, i32, u64)] = &[
        // a.sort_bits < b.sort_bits  -> first `if` true
        (0, 0, 1, 0),
        (1, 5, 2, 1),
        // a.sort_bits == b.sort_bits -> first `if` still true (second is dead)
        (7, 0, 7, 0),
        (7, 100, 7, 1),
        (7, 1, 7, 100),
        // a.sort_bits > b.sort_bits  -> first `if` false, second `if` false
        (2, 0, 1, 0),
        (1, 1, 0, 100),
        // signed comparisons
        (-1, 0, 0, 0),
        (0, 0, -1, 0),
        (i32::MIN, 0, i32::MAX, 0),
        (i32::MAX, 0, i32::MIN, 0),
        (i32::MIN, u64::MAX, i32::MIN, 0),
        // unsigned 64-bit texture ids at the extremes
        (0, u64::MAX, 0, 0),
        (0, 0, 0, u64::MAX),
        (0, u64::MAX, 0, u64::MAX),
        (5, u64::MAX, 5, u64::MAX - 1),
        (i32::MAX, u64::MAX, i32::MAX, u64::MAX),
    ];

    for (i, &(sa, ta, sb, tb)) in cases.iter().enumerate() {
        let input = [Sprite::new(ta, sa), Sprite::new(tb, sb)];
        impls.assert_matches(&input, 2, &format!("comparator case {i}"));
    }
}

// ---------------------------------------------------------------------------
// Level 2: merge iteration branches (i < split / j >= hi / else)
// ---------------------------------------------------------------------------

#[test]
fn merge_iteration_small_sizes() {
    let impls = Impls::load();

    // Sizes 2..=4 give split points 1 and 2, covering: left run exhausted
    // first (`i < split` false), right run exhausted first (`j >= hi` true),
    // and interleaving.
    for size in 2..=4i32 {
        let n = size as usize;
        // Every permutation of distinct descending/ascending sort_bits, plus
        // ties, expressed as a base-`n` counter over sort_bits values 0..n.
        let total = (n as u32).pow(n as u32);
        for code in 0..total {
            let mut c = code;
            let mut input = Vec::with_capacity(n);
            for idx in 0..n {
                let bits = (c % n as u32) as i32;
                c /= n as u32;
                // texture_id encodes the original position so ties are visible.
                input.push(Sprite::new(idx as u64 + 1, bits));
            }
            impls.assert_matches(&input, size, &format!("iter size={size} code={code}"));
        }
    }
}

#[test]
fn merge_iteration_ties_preserve_c_behaviour() {
    let impls = Impls::load();

    // All-equal sort_bits with descending texture ids: the dead second `if`
    // in the C comparator means texture_id never breaks ties, so the C output
    // is *not* sorted by texture_id. Rust must reproduce that.
    for size in 1..=33i32 {
        let input: Vec<Sprite> = (0..size)
            .map(|i| Sprite::new((size - i) as u64, 0))
            .collect();
        impls.assert_matches(&input, size, &format!("all-ties size={size}"));
    }
}

// ---------------------------------------------------------------------------
// Level 3: recursion / buffer role swapping across every small size
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_sizes_structured_patterns() {
    let impls = Impls::load();

    for size in 0..=80i32 {
        let n = size as usize;

        let patterns: Vec<(&str, Vec<Sprite>)> = vec![
            (
                "ascending",
                (0..n).map(|i| Sprite::new(i as u64, i as i32)).collect(),
            ),
            (
                "descending",
                (0..n)
                    .map(|i| Sprite::new(i as u64, (n - i) as i32))
                    .collect(),
            ),
            (
                "all-equal",
                (0..n).map(|i| Sprite::new(i as u64, 42)).collect(),
            ),
            (
                "two-values",
                (0..n)
                    .map(|i| Sprite::new(i as u64, (i % 2) as i32))
                    .collect(),
            ),
            (
                "sawtooth",
                (0..n)
                    .map(|i| Sprite::new(i as u64, (i % 7) as i32 - 3))
                    .collect(),
            ),
            (
                "organ-pipe",
                (0..n)
                    .map(|i| {
                        let v = if i < n / 2 { i } else { n - i };
                        Sprite::new(i as u64, v as i32)
                    })
                    .collect(),
            ),
            (
                "extremes",
                (0..n)
                    .map(|i| {
                        let bits = match i % 4 {
                            0 => i32::MIN,
                            1 => i32::MAX,
                            2 => 0,
                            _ => -1,
                        };
                        let tex = match i % 3 {
                            0 => u64::MAX,
                            1 => 0,
                            _ => 1 << 63,
                        };
                        Sprite::new(tex, bits)
                    })
                    .collect(),
            ),
        ];

        for (name, input) in patterns {
            impls.assert_matches(&input, size, &format!("{name} size={size}"));
        }
    }
}

#[test]
fn degenerate_sizes() {
    let impls = Impls::load();

    // size == 0: C memcpy's 0 bytes and the recursion returns immediately.
    impls.assert_matches(&[], 0, "empty");
    // A non-empty allocation driven with size == 0 must be left untouched.
    let input = vec![Sprite::new(9, -9); 4];
    let ((c_a, c_b), (r_a, r_b)) = impls.run_both(&input, 0);
    assert_eq!(common::as_bytes(&c_a), common::as_bytes(&r_a));
    assert_eq!(common::as_bytes(&c_b), common::as_bytes(&r_b));
    assert_eq!(c_a, input, "C must not touch `a` when size == 0");

    // size == 1: memcpy one element, recursion returns (hi - lo == 1).
    impls.assert_matches(&[Sprite::new(u64::MAX, i32::MIN)], 1, "single");
}

#[test]
fn size_smaller_than_buffer_leaves_tail_untouched() {
    let impls = Impls::load();

    // Sentinel tail beyond `size` must be identical afterwards in both impls.
    for size in 0..=17i32 {
        let n = size as usize;
        let tail = 5usize;
        let mut input: Vec<Sprite> = (0..n)
            .map(|i| Sprite::new(i as u64, (n - i) as i32))
            .collect();
        for t in 0..tail {
            input.push(Sprite::new(0xDEAD_BEEF_0000_0000 + t as u64, 0x7F0F0F0F));
        }

        let ((c_a, c_b), (r_a, r_b)) = impls.run_both(&input, size);
        assert_eq!(
            common::as_bytes(&c_a),
            common::as_bytes(&r_a),
            "tail test `a` size={size}"
        );
        assert_eq!(
            common::as_bytes(&c_b),
            common::as_bytes(&r_b),
            "tail test `b` size={size}"
        );
        assert_eq!(
            &c_a[n..],
            &input[n..],
            "C overwrote past `size` in `a` (size={size})"
        );
    }
}

// ---------------------------------------------------------------------------
// Level 4: randomised differential fuzzing of the public entry point
// ---------------------------------------------------------------------------

#[test]
fn fuzz_random_inputs() {
    let impls = Impls::load();
    let seed: u64 = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xC0FFEE_1234_5678);
    let iters: u32 = std::env::var("FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let mut rng = Rng::new(seed);

    for iter in 0..iters {
        let size = rng.below(129) as i32;
        let n = size as usize;
        // Vary the entropy of sort_bits so ties are common in some rounds.
        let bits_mod = match iter % 5 {
            0 => 1u64,
            1 => 2,
            2 => 8,
            3 => 1000,
            _ => 0, // 0 => full-range i32
        };

        let input: Vec<Sprite> = (0..n)
            .map(|_| {
                let bits = if bits_mod == 0 {
                    rng.next_i32()
                } else {
                    rng.below(bits_mod) as i32 - (bits_mod / 2) as i32
                };
                let tex = match iter % 3 {
                    0 => rng.next_u64(),
                    1 => rng.below(4),
                    _ => rng.next_u64() | (1 << 63),
                };
                Sprite::new(tex, bits)
            })
            .collect();

        impls.assert_matches(&input, size, &format!("fuzz iter={iter} size={size}"));
    }
}

#[test]
fn fuzz_larger_inputs() {
    let impls = Impls::load();
    let mut rng = Rng::new(0xABCD_EF01_2345_6789);

    for size in [255i32, 256, 257, 511, 512, 513, 1000, 1024, 4096, 10_007] {
        for round in 0..4 {
            let n = size as usize;
            let input: Vec<Sprite> = (0..n)
                .map(|i| {
                    let bits = match round {
                        0 => rng.next_i32(),
                        1 => (rng.below(4)) as i32,
                        2 => i as i32,
                        _ => -(i as i32),
                    };
                    Sprite::new(rng.next_u64(), bits)
                })
                .collect();
            impls.assert_matches(&input, size, &format!("large size={size} round={round}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Aliasing / in-place style calls that the C code tolerates
// ---------------------------------------------------------------------------

#[test]
fn repeated_calls_are_stable() {
    let impls = Impls::load();
    let mut rng = Rng::new(7);

    // Feed the previous output back in; divergence compounds quickly if the
    // buffer role swapping differs at all.
    for size in [3i32, 8, 13, 16, 31, 64, 65] {
        let n = size as usize;
        let mut c_a: Vec<Sprite> = (0..n)
            .map(|_| Sprite::new(rng.next_u64(), rng.below(16) as i32 - 8))
            .collect();
        let mut r_a = c_a.clone();
        let mut c_b = vec![Sprite::new(0, 0); n];
        let mut r_b = vec![Sprite::new(0, 0); n];

        for round in 0..6 {
            unsafe {
                (impls.c_merge_sort)(c_a.as_mut_ptr(), c_b.as_mut_ptr(), size);
                (impls.rust_merge_sort)(r_a.as_mut_ptr(), r_b.as_mut_ptr(), size);
            }
            assert_eq!(
                common::as_bytes(&c_a),
                common::as_bytes(&r_a),
                "repeat `a` size={size} round={round}"
            );
            assert_eq!(
                common::as_bytes(&c_b),
                common::as_bytes(&r_b),
                "repeat `b` size={size} round={round}"
            );
        }
    }
}
