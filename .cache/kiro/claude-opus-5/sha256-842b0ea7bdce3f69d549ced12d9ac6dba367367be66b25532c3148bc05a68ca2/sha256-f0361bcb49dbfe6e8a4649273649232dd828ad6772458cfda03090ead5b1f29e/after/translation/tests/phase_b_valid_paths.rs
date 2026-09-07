//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares all `16 * size` output bytes of
//! **both** the sorted buffer and the scratch buffer.

mod common;

use common::*;

/// Number of randomized inputs per (size, shape) combination.
const REPS: usize = 24;

fn sweep_sizes(label: &str, sizes: &[usize], shapes: &[Shape], reps: usize) {
    let mut rng = Rng::new(RNG_SEED);
    for &n in sizes {
        for &shape in shapes {
            for r in 0..reps {
                // Alternate padding treatment across reps: zero, fixed 0xAA,
                // random bytes (axis G).
                let pad = match r % 3 {
                    0 => None,
                    1 => Some(0xAA),
                    _ => Some(0xFF), // sentinel => random padding per element
                };
                let a = make_sprites(shape, n, &mut rng, pad);
                let b_fill = [0x00u8, 0x55, 0xC3][r % 3];
                let case = Case::from_a(
                    format!("{label} n={n} shape={} rep={r}", shape.name()),
                    a,
                    b_fill,
                );
                assert_same(&case);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 1 — size = 0, scratch garbage-filled
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_size_zero() {
    let mut rng = Rng::new(RNG_SEED);
    for r in 0..REPS {
        for &fill in &[0x00u8, 0x55, 0xFF, 0xC3] {
            let a = make_sprites(Shape::Random, 0, &mut rng, Some(0xAA));
            let case = Case::new(
                format!("size0 rep={r} fill={fill:#02x}"),
                a,
                vec![Sprite([fill; SPRITE_SIZE]); 4],
                0,
            );
            assert_same(&case);
        }
    }
    // Also: non-empty allocation but size argument 0 — nothing may be touched.
    let a = make_sprites(Shape::Random, 8, &mut rng, Some(0xAA));
    let case = Case::new("alloc8_size0", a, vec![Sprite([0x5A; SPRITE_SIZE]); 8], 0);
    assert_same(&case);
}

// ---------------------------------------------------------------------------
// Row 2 — size = 1
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_size_one() {
    sweep_sizes("row2", &[1], &Shape::sweep_set(), REPS);
    // size argument 1 while the allocation is larger: elements 1.. must be
    // untouched in both buffers.
    let mut rng = Rng::new(RNG_SEED ^ 2);
    for r in 0..REPS {
        let a = make_sprites(Shape::Random, 6, &mut rng, Some(0xFF));
        let case = Case::new(
            format!("alloc6_size1 rep={r}"),
            a,
            vec![Sprite([0x99; SPRITE_SIZE]); 6],
            1,
        );
        assert_same(&case);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — size = 2 (single merge of two length-1 runs)
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_size_two() {
    sweep_sizes("row3", &[2], &Shape::sweep_set(), REPS * 2);
    // Exhaustively cover the two orderings and the tie, with padding garbage.
    let cases: [(i32, i32, u64, u64); 6] = [
        (0, 1, 7, 9),
        (1, 0, 7, 9),
        (5, 5, 1, 2),
        (5, 5, 2, 1),
        (i32::MIN, i32::MAX, 0, u64::MAX),
        (i32::MAX, i32::MIN, u64::MAX, 0),
    ];
    for (i, &(b0, b1, t0, t1)) in cases.iter().enumerate() {
        let a = vec![Sprite::new(t0, b0, 0xAA), Sprite::new(t1, b1, 0xBB)];
        assert_same(&Case::from_a(format!("size2_exhaustive#{i}"), a, 0x77));
    }
}

// ---------------------------------------------------------------------------
// Row 4 — size = 3 (odd → uneven 1/2 split)
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_size_three() {
    sweep_sizes("row4", &[3], &Shape::sweep_set(), REPS * 2);
    // All 3! orderings of three distinct keys, plus all-tie permutations.
    for perm in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        let a: Vec<Sprite> = perm
            .iter()
            .enumerate()
            .map(|(i, &k)| Sprite::new(100 + i as u64, k as i32, 0xAA))
            .collect();
        assert_same(&Case::from_a(format!("size3_perm{perm:?}"), a, 0x11));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — size = 4 (power of two, balanced splits)
// ---------------------------------------------------------------------------
#[test]
fn cfg_05_size_four() {
    sweep_sizes("row5", &[4], &Shape::sweep_set(), REPS * 2);
    // All 4! = 24 orderings of four distinct keys.
    let keys = [0i32, 1, 2, 3];
    let mut perm = keys;
    permute(&mut perm, 0, &mut |p: &[i32]| {
        let a: Vec<Sprite> = p
            .iter()
            .enumerate()
            .map(|(i, &k)| Sprite::new(200 + i as u64, k, 0x3C))
            .collect();
        assert_same(&Case::from_a(format!("size4_perm{p:?}"), a, 0x22));
    });
}

fn permute(v: &mut [i32], k: usize, f: &mut impl FnMut(&[i32])) {
    if k == v.len() {
        f(v);
        return;
    }
    for i in k..v.len() {
        v.swap(k, i);
        permute(v, k + 1, f);
        v.swap(k, i);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — sizes 5, 6, 7
// ---------------------------------------------------------------------------
#[test]
fn cfg_06_sizes_five_six_seven() {
    sweep_sizes("row6", &[5, 6, 7], &Shape::sweep_set(), REPS);
}

// ---------------------------------------------------------------------------
// Row 7 — powers of two
// ---------------------------------------------------------------------------
#[test]
fn cfg_07_powers_of_two() {
    sweep_sizes("row7", &[8, 16, 32, 64, 128, 256], &Shape::sweep_set(), 8);
}

// ---------------------------------------------------------------------------
// Row 8 — one off powers of two (worst-case ragged splits)
// ---------------------------------------------------------------------------
#[test]
fn cfg_08_off_by_one_powers() {
    sweep_sizes(
        "row8",
        &[9, 17, 33, 63, 65, 127, 129, 255, 257],
        &Shape::sweep_set(),
        6,
    );
}

// ---------------------------------------------------------------------------
// Row 9 — size 1000, deep recursion
// ---------------------------------------------------------------------------
#[test]
fn cfg_09_size_1000() {
    sweep_sizes("row9", &[1000], &Shape::sweep_set(), 4);
}

// ---------------------------------------------------------------------------
// Row 10 — size 4096, deep recursion + large memcpy
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_size_4096() {
    sweep_sizes("row10", &[4096], &[Shape::Random, Shape::FewBuckets(7)], 3);
}

// ---------------------------------------------------------------------------
// Row 11 — broad property sweep: random size in 1..=512, random data
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_random_size_property_sweep() {
    let mut rng = Rng::new(RNG_SEED ^ 0x11);
    for it in 0..200 {
        let n = 1 + rng.below(512);
        let pad = match it % 3 {
            0 => None,
            1 => Some(0xAA),
            _ => Some(0xFF),
        };
        let a = make_sprites(Shape::Random, n, &mut rng, pad);
        assert_same(&Case::from_a(
            format!("prop it={it} n={n}"),
            a,
            (it % 251) as u8,
        ));
    }
}

// ---------------------------------------------------------------------------
// Row 12 — already ascending (left run always exhausts first)
// ---------------------------------------------------------------------------
#[test]
fn cfg_12_already_ascending() {
    sweep_sizes("row12", &(2..=64).collect::<Vec<_>>(), &[Shape::Ascending], 3);
}

// ---------------------------------------------------------------------------
// Row 13 — already descending (right run drains first)
// ---------------------------------------------------------------------------
#[test]
fn cfg_13_already_descending() {
    sweep_sizes(
        "row13",
        &(2..=64).collect::<Vec<_>>(),
        &[Shape::Descending],
        3,
    );
}

// ---------------------------------------------------------------------------
// Row 14 — all sort_bits equal, texture_id ascending
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_all_equal_tex_ascending() {
    sweep_sizes(
        "row14",
        &[2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100],
        &[Shape::AllEqualTexAsc],
        4,
    );
}

// ---------------------------------------------------------------------------
// Row 15 — all sort_bits equal, texture_id DESCENDING (dead-tiebreak probe)
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_all_equal_tex_descending() {
    sweep_sizes(
        "row15",
        &[2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100],
        &[Shape::AllEqualTexDesc],
        4,
    );
}

// ---------------------------------------------------------------------------
// Row 16 — few distinct sort_bits (many ties)
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_few_buckets_many_ties() {
    for k in [2u32, 3, 5] {
        sweep_sizes(
            &format!("row16 k={k}"),
            &[2, 3, 5, 8, 13, 21, 34, 55, 89, 144],
            &[Shape::FewBuckets(k)],
            4,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 17 — all sort_bits negative
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_all_negative_bits() {
    sweep_sizes(
        "row17",
        &[1, 2, 3, 4, 8, 16, 17, 33, 64, 129],
        &[Shape::AllNegative],
        6,
    );
}

// ---------------------------------------------------------------------------
// Row 18 — mixed sign, full i32 range
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_mixed_sign_full_range() {
    sweep_sizes(
        "row18",
        &[1, 2, 3, 4, 8, 16, 17, 33, 64, 129, 256],
        &[Shape::Random],
        6,
    );
}

// ---------------------------------------------------------------------------
// Row 19 — sort_bits from {INT_MIN, -1, 0, 1, INT_MAX}
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_boundary_sort_bits() {
    sweep_sizes(
        "row19",
        &[1, 2, 3, 4, 5, 8, 16, 17, 32, 64, 100],
        &[Shape::BoundaryBits],
        8,
    );
    // Every ordered pair of boundary values at size 2.
    const BITS: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];
    for &x in &BITS {
        for &y in &BITS {
            let a = vec![Sprite::new(1, x, 0xAA), Sprite::new(2, y, 0xBB)];
            assert_same(&Case::from_a(format!("bits_pair({x},{y})"), a, 0x33));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — texture_id extremes with all sort_bits equal
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_boundary_texture_ids() {
    sweep_sizes(
        "row20",
        &[1, 2, 3, 4, 5, 8, 16, 17, 32, 64],
        &[Shape::BoundaryTex],
        8,
    );
    const TEX: [u64; 5] = [0, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX];
    for &x in &TEX {
        for &y in &TEX {
            let a = vec![Sprite::new(x, 42, 0xAA), Sprite::new(y, 42, 0xBB)];
            assert_same(&Case::from_a(format!("tex_pair({x},{y})"), a, 0x44));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — scratch buffer pre-zeroed vs pre-patterned
// ---------------------------------------------------------------------------
#[test]
fn cfg_21_scratch_buffer_prefill() {
    let mut rng = Rng::new(RNG_SEED ^ 0x21);
    for n in 0..=32usize {
        for &fill in &[0x00u8, 0x55, 0xAA, 0xFF, 0x01] {
            for r in 0..3 {
                let a = make_sprites(Shape::Random, n, &mut rng, Some(0xFF));
                assert_same(&Case::from_a(
                    format!("scratch n={n} fill={fill:#02x} rep={r}"),
                    a,
                    fill,
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 22 & 23 — padding bytes non-zero / zero
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_padding_nonzero() {
    let mut rng = Rng::new(RNG_SEED ^ 0x22);
    for n in 1..=32usize {
        for &p in &[0xAAu8, 0x01, 0xFE, 0x80] {
            let a = make_sprites(Shape::Random, n, &mut rng, Some(p));
            assert_same(&Case::from_a(format!("pad n={n} p={p:#02x}"), a, 0x55));
        }
        // Fully random padding, distinct per element.
        for r in 0..4 {
            let a = make_sprites(Shape::FewBuckets(3), n, &mut rng, Some(0xFF));
            assert_same(&Case::from_a(format!("padrand n={n} rep={r}"), a, 0x55));
        }
    }
}

#[test]
fn cfg_23_padding_zero_control() {
    let mut rng = Rng::new(RNG_SEED ^ 0x23);
    for n in 0..=32usize {
        for r in 0..4 {
            let a = make_sprites(Shape::Random, n, &mut rng, None);
            let case = Case::from_a(format!("padzero n={n} rep={r}"), a, 0x00);
            assert_same(&case);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — repeated invocation on the same buffers
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_repeated_invocation() {
    let p = common::pair();
    let mut rng = Rng::new(RNG_SEED ^ 0x24);
    for n in [0usize, 1, 2, 3, 5, 8, 17, 64, 129] {
        for r in 0..6 {
            let a0 = make_sprites(Shape::FewBuckets(4), n, &mut rng, Some(0xFF));
            let b0 = vec![Sprite([0x5A; SPRITE_SIZE]); n];

            let mut ac = a0.clone();
            let mut bc = b0.clone();
            let mut ar = a0.clone();
            let mut br = b0.clone();

            for call in 0..3 {
                unsafe {
                    p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), n as i32);
                    p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), n as i32);
                }
                assert_eq!(
                    ac.as_slice(),
                    ar.as_slice(),
                    "buffer a diverged after call {call} (n={n}, rep={r})"
                );
                assert_eq!(
                    bc.as_slice(),
                    br.as_slice(),
                    "buffer b diverged after call {call} (n={n}, rep={r})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — full cross-product sweep: size 0..=80 x 6 shapes
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_full_cross_product() {
    let mut rng = Rng::new(RNG_SEED ^ 0x25);
    for n in 0..=80usize {
        for &shape in &Shape::sweep_set() {
            for r in 0..3 {
                let pad = match r {
                    0 => None,
                    1 => Some(0xAA),
                    _ => Some(0xFF),
                };
                let a = make_sprites(shape, n, &mut rng, pad);
                assert_same(&Case::from_a(
                    format!("xprod n={n} shape={} rep={r}", shape.name()),
                    a,
                    [0x00u8, 0x55, 0xC3][r],
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — a and b adjacent in one allocation, with trailing guard
// ---------------------------------------------------------------------------
#[test]
fn cfg_26_adjacent_buffers_with_guard() {
    let mut rng = Rng::new(RNG_SEED ^ 0x26);
    for n in 1..=64usize {
        for &shape in &[Shape::Random, Shape::Ascending, Shape::Descending] {
            for r in 0..2 {
                let a = make_sprites(shape, n, &mut rng, Some(0xFF));
                assert_same_adjacent(
                    &format!("adj n={n} shape={} rep={r}", shape.name()),
                    &a,
                    n as i32,
                    0x7E,
                );
            }
        }
    }
}
