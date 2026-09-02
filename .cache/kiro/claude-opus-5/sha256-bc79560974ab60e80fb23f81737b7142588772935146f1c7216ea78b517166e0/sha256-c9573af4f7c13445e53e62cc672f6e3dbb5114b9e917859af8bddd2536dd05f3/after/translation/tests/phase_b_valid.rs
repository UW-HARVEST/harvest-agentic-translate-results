//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through their exported `dataentry` symbol and compares the returned `int`
//! byte-for-byte. All randomized rows use a fixed seed.

mod common;

use common::{pair, Rng, EDGE_INTS};

// ---------------------------------------------------------------------------
// Row 1 — mode 1, default count 5, param2 over every valid index
// ---------------------------------------------------------------------------
#[test]
fn row01_mode1_default_count_all_indices() {
    let p = pair();
    for param1 in [0, -1, -5, -1000, i32::MIN, i32::MIN + 1] {
        for param2 in 0..5 {
            for param3 in [0, 7, -7, i32::MAX, i32::MIN] {
                p.assert_same(1, param1, param2, param3);
            }
        }
    }
    // The C picks count = 5 and ids 100..104, so value = (100+param2)*10.
    p.assert_same_and_eq(1, 0, 0, 0, 1000);
    p.assert_same_and_eq(1, 0, 4, 0, 1040);
}

// ---------------------------------------------------------------------------
// Row 2 — mode 1, count 1 (single element: first == last)
// ---------------------------------------------------------------------------
#[test]
fn row02_mode1_single_element() {
    let p = pair();
    p.assert_same_and_eq(1, 1, 0, 0, 1000);
    let mut rng = Rng::new(0x1002);
    for _ in 0..2000 {
        p.assert_same(1, 1, rng.range(-3, 3), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 3 — mode 1, count 2, boundary index pair
// ---------------------------------------------------------------------------
#[test]
fn row03_mode1_count_two_boundaries() {
    let p = pair();
    p.assert_same_and_eq(1, 2, 0, 0, 1000);
    p.assert_same_and_eq(1, 2, 1, 0, 1010);
    let mut rng = Rng::new(0x1003);
    for _ in 0..2000 {
        p.assert_same(1, 2, rng.range(-4, 4), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 4 — mode 1, count == MAX_ENTRIES (10), all indices
// ---------------------------------------------------------------------------
#[test]
fn row04_mode1_max_entries() {
    let p = pair();
    for param2 in 0..10 {
        let want = (100 + param2) * 10;
        p.assert_same_and_eq(1, 10, param2, 0, want);
    }
    let mut rng = Rng::new(0x1004);
    for _ in 0..5000 {
        p.assert_same(1, 10, rng.range(-12, 12), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 5 — mode 1, many entries: 3/4/5-digit `Entry_%d` names
// ---------------------------------------------------------------------------
#[test]
fn row05_mode1_many_entries_name_widths() {
    let p = pair();
    let mut rng = Rng::new(0x1005);
    for count in [100, 900, 901, 1000, 9900, 10_000, 20_000] {
        // First, last, and the id-width transitions (999->1000, 9999->10000).
        let mut idxs = vec![0, count - 1, count / 2];
        for boundary in [899, 900, 901, 9899, 9900, 9901] {
            if boundary < count {
                idxs.push(boundary);
            }
        }
        for _ in 0..24 {
            idxs.push(rng.range(0, count - 1));
        }
        for idx in idxs {
            p.assert_same_and_eq(1, count, idx, 0, (100 + idx) * 10);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — mode 1, randomized count and index (in- and out-of-range mixed)
// ---------------------------------------------------------------------------
#[test]
fn row06_mode1_randomized_count_and_index() {
    let p = pair();
    let mut rng = Rng::new(0x1006);
    for _ in 0..200_000 {
        let param1 = rng.range(-8, 64);
        let param2 = match rng.next_u64() % 3 {
            0 => rng.range(-4, 68),
            1 => rng.spicy_i32(),
            _ => rng.range(0, 63),
        };
        p.assert_same(1, param1, param2, rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 7 — mode 1, param3 must be ignored on this path
// ---------------------------------------------------------------------------
#[test]
fn row07_mode1_param3_ignored() {
    let p = pair();
    let mut rng = Rng::new(0x1007);
    let baseline = p.assert_same(1, 7, 3, 0);
    for _ in 0..3000 {
        let got = p.assert_same(1, 7, 3, rng.spicy_i32());
        assert_eq!(got, baseline, "param3 must not affect mode 1");
    }
    for e in EDGE_INTS {
        assert_eq!(p.assert_same(1, 7, 3, e), baseline);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — mode 2, default count 3, identity multiplier
// ---------------------------------------------------------------------------
#[test]
fn row08_mode2_default_count_identity() {
    let p = pair();
    // ids 200,201,202 -> values 2000,2010,2020 -> total 6030 (multiplier 1).
    p.assert_same_and_eq(2, 0, 1, 0, 6030);
    for param1 in [0, -1, -3, -100_000, i32::MIN, i32::MIN + 1] {
        p.assert_same_and_eq(2, param1, 1, 0, 6030);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — mode 2, small counts, small positive multiplier
// ---------------------------------------------------------------------------
#[test]
fn row09_mode2_small_counts() {
    let p = pair();
    for count in [1, 2, 3, 4, 5, 9, 10, 11] {
        for mult in [1, 2, 3, 7, 100] {
            let mut want: i32 = 0;
            for i in 0..count {
                want = want.wrapping_add(((200 + i) * 10i32).wrapping_mul(mult));
            }
            p.assert_same_and_eq(2, count, mult, 0, want);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — mode 2, multiplier 0 -> total 0 -> param3 NOT added
// ---------------------------------------------------------------------------
#[test]
fn row10_mode2_zero_multiplier_skips_param3() {
    let p = pair();
    let mut rng = Rng::new(0x1010);
    for count in [-1, 0, 1, 2, 3, 10, 500] {
        for _ in 0..200 {
            p.assert_same_and_eq(2, count, 0, rng.spicy_i32(), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — mode 2, negative multiplier
// ---------------------------------------------------------------------------
#[test]
fn row11_mode2_negative_multiplier() {
    let p = pair();
    let mut rng = Rng::new(0x1011);
    for mult in [-1, -2, -7, -1000, -100_000, i32::MIN, i32::MIN + 1] {
        for count in [1, 2, 3, 10, 47] {
            for _ in 0..40 {
                p.assert_same(2, count, mult, rng.spicy_i32());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — mode 2, multiplier large enough that `total` wraps
// ---------------------------------------------------------------------------
#[test]
fn row12_mode2_total_overflow() {
    let p = pair();
    let mut rng = Rng::new(0x1012);
    for mult in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2,
        1_000_000,
        123_456_789,
        0x4000_0000,
    ] {
        for count in [1, 2, 3, 10, 100, 1000] {
            for _ in 0..20 {
                p.assert_same(2, count, mult, rng.spicy_i32());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — mode 2, many entries (long wrapping accumulation)
// ---------------------------------------------------------------------------
#[test]
fn row13_mode2_many_entries() {
    let p = pair();
    let mut rng = Rng::new(0x1013);
    for count in [100, 999, 1000, 1001, 9999, 10_000, 20_000] {
        for _ in 0..12 {
            p.assert_same(2, count, rng.spicy_i32(), rng.spicy_i32());
        }
        for mult in [1, -1, 3, 65_536] {
            p.assert_same(2, count, mult, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — mode 2, param3 at the int boundaries (overflow in `result += p3`)
// ---------------------------------------------------------------------------
#[test]
fn row14_mode2_param3_overflow() {
    let p = pair();
    for param3 in EDGE_INTS {
        for count in [1, 2, 3, 10, 100] {
            for mult in [1, -1, 3, i32::MAX, i32::MIN] {
                p.assert_same(2, count, mult, param3);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — mode 2, fully randomized
// ---------------------------------------------------------------------------
#[test]
fn row15_mode2_randomized() {
    let p = pair();
    let mut rng = Rng::new(0x1015);
    for _ in 0..200_000 {
        p.assert_same(2, rng.range(-8, 64), rng.spicy_i32(), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 16 — mode 3, full 4x3 valid cross-product, param3 == 0
// ---------------------------------------------------------------------------
#[test]
fn row16_mode3_full_table() {
    let p = pair();
    let table = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
    for row in 0..4usize {
        for col in 0..3usize {
            p.assert_same_and_eq(3, row as i32, col as i32, 0, table[row][col] * 2);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — mode 3, full cross-product with randomized param3
// ---------------------------------------------------------------------------
#[test]
fn row17_mode3_full_table_randomized_param3() {
    let p = pair();
    let mut rng = Rng::new(0x1017);
    for _ in 0..200_000 {
        p.assert_same(3, rng.range(0, 3), rng.range(0, 2), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 18 — mode 3, param3 at INT_MAX / INT_MIN (additive overflow)
// ---------------------------------------------------------------------------
#[test]
fn row18_mode3_param3_overflow() {
    let p = pair();
    for row in 0..4 {
        for col in 0..3 {
            for param3 in EDGE_INTS {
                p.assert_same(3, row, col, param3);
            }
        }
    }
    // 120*2 = 240; 240 + INT_MAX wraps.
    p.assert_same_and_eq(3, 3, 2, 0, 240);
}

// ---------------------------------------------------------------------------
// Row 19 — default arm via mode 0
// ---------------------------------------------------------------------------
#[test]
fn row19_default_arm_mode_zero() {
    let p = pair();
    let mut rng = Rng::new(0x1019);
    p.assert_same_and_eq(0, 1, 0, 0, 8);
    p.assert_same_and_eq(0, 3, 0, 0, 24);
    for _ in 0..200_000 {
        p.assert_same(0, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 20 — default arm via every other out-of-range mode
// ---------------------------------------------------------------------------
#[test]
fn row20_default_arm_other_modes() {
    let p = pair();
    let mut rng = Rng::new(0x1020);
    let modes = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -3,
        -2,
        -1,
        0,
        4,
        5,
        6,
        100,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for mode in modes {
        p.assert_same_and_eq(mode, 1, 0, 0, 8);
        for _ in 0..1000 {
            p.assert_same(mode, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — default arm, param1 zero and at the int boundaries
// ---------------------------------------------------------------------------
#[test]
fn row21_default_arm_param1_edges() {
    let p = pair();
    p.assert_same_and_eq(0, 0, 0, 0, 0);
    for mode in [i32::MIN, -7, 0, 4, i32::MAX] {
        for param1 in EDGE_INTS {
            // 8 * param1 with signed wraparound.
            p.assert_same_and_eq(mode, param1, 0, 0, 8i32.wrapping_mul(param1));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — full randomized 4-tuple sweep across all arms
// ---------------------------------------------------------------------------
#[test]
fn row22_randomized_all_modes() {
    let p = pair();
    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..1_000_000 {
        // Modes 1 and 2 allocate `param1 * 40` bytes, so param1 is bounded for
        // them; row 25 covers the huge-`param1` allocation-failure path.
        let mode = match rng.next_u64() % 6 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 0,
            4 => rng.range(-4, 8),
            _ => rng.spicy_i32(),
        };
        let param1 = if mode == 1 || mode == 2 {
            rng.range(-8, 48)
        } else {
            rng.spicy_i32()
        };
        p.assert_same(mode, param1, rng.spicy_i32(), rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 23 — exhaustive small-domain sweep
// ---------------------------------------------------------------------------
#[test]
fn row23_exhaustive_small_domain() {
    let p = pair();
    for mode in -6..=12 {
        for param1 in -6..=20 {
            for param2 in -6..=24 {
                for param3 in [-6, -1, 0, 1, 12] {
                    p.assert_same(mode, param1, param2, param3);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — every parameter at the int boundaries (cross-product)
// ---------------------------------------------------------------------------
#[test]
fn row24_boundary_cross_product() {
    let p = pair();
    // param1 drives the allocation size in modes 1/2, so keep the cross-product
    // over modes 0/3/huge-mode where it is free, and cover modes 1/2 with
    // bounded param1 plus the dedicated row 25 test.
    let edges = EDGE_INTS;
    for mode in [i32::MIN, -1, 0, 3, 4, i32::MAX] {
        for a in edges {
            for b in edges {
                for c in [i32::MIN, -1, 0, 1, i32::MAX] {
                    p.assert_same(mode, a, b, c);
                }
            }
        }
    }
    for mode in [1, 2] {
        for a in [-1000, -1, 0, 1, 2, 3, 10] {
            for b in edges {
                for c in [i32::MIN, -1, 0, 1, i32::MAX] {
                    p.assert_same(mode, a, b, c);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — huge param1: the allocation-failure path in `create_entries`
// ---------------------------------------------------------------------------
#[test]
fn row25_huge_count_allocation_failure() {
    let p = pair();
    for mode in [1, 2] {
        for param1 in [
            i32::MAX,
            i32::MAX - 1,
            i32::MAX / 2,
            0x4000_0000,
            0x2000_0000,
            1_000_000_000,
        ] {
            p.assert_same_and_eq(mode, param1, 1, 1, -1);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — allocator-threshold agreement.
//
// `create_entries` decides success purely on what `malloc(count * 40)` returns,
// so the count at which the library flips from "works" to "-1" is a property of
// the allocator. The Rust translation must flip at the SAME count, not merely
// "fail somewhere". A power-of-two ladder crosses the boundary; the succeeding
// side is capped so the test stays fast (each successful count writes
// `count * 40` bytes on both sides).
// ---------------------------------------------------------------------------
#[test]
fn row26_allocation_threshold_agreement() {
    let p = pair();
    let mut last_ok: Option<i32> = None;
    let mut first_fail: Option<i32> = None;

    for exp in 10..=31u32 {
        let count = if exp == 31 { i32::MAX } else { 1i32 << exp };
        // Skip the multi-second successful allocations above 2^24 (~640 MiB);
        // the transition itself is what matters and it lies far above that.
        if (10..=24).contains(&exp) || exp >= 27 {
            let got = p.assert_same(1, count, 0, 0);
            if got == -1 {
                first_fail = first_fail.or(Some(count));
            } else {
                assert_eq!(got, 1000, "id 100 -> value 1000 for count={count}");
                last_ok = Some(count);
            }
            // Same ladder through mode 2, which uses the other base id.
            p.assert_same(2, count, 0, 0);
        }
    }

    assert!(
        last_ok.is_some(),
        "at least one count on the ladder must allocate successfully"
    );
    assert!(
        first_fail.is_some(),
        "at least one count on the ladder must exhaust the allocator"
    );
    // And the two implementations must agree on both sides of wherever the
    // real boundary sits on this machine.
    assert!(last_ok.unwrap() < first_fail.unwrap());
}
