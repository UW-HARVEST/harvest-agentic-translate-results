//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the returned `int`
//! byte-for-byte, over many randomized inputs with a fixed per-row seed.

mod common;
use common::{Pair, Rng};

fn pair() -> Pair {
    Pair::load()
}

// ---------------------------------------------------------------------------
// Rows 1-9: mode 1 — create_entries(base_id=100) + find_entry + free
// ---------------------------------------------------------------------------

#[test]
fn row01_mode1_single_entry_hit() {
    let p = pair();
    // param1 == 1 -> exactly one entry with id 100; param2 == 0 hits it.
    let r = p.assert_same(1, 1, 0, 0);
    assert_eq!(r, 1000, "sanity: entry 100 has value 1000");
    let mut rng = Rng::new(0x0101);
    for _ in 0..2_000 {
        p.assert_same(1, 1, 0, rng.spicy_i32());
    }
}

#[test]
fn row02_mode1_first_element() {
    let p = pair();
    let mut rng = Rng::new(0x0102);
    for _ in 0..5_000 {
        let count = rng.range_i32(2, 10);
        p.assert_same(1, count, 0, rng.spicy_i32());
    }
}

#[test]
fn row03_mode1_last_element() {
    let p = pair();
    let mut rng = Rng::new(0x0103);
    for _ in 0..5_000 {
        let count = rng.range_i32(2, 10);
        p.assert_same(1, count, count - 1, rng.spicy_i32());
    }
}

#[test]
fn row04_mode1_middle_element() {
    let p = pair();
    let mut rng = Rng::new(0x0104);
    for _ in 0..5_000 {
        let count = rng.range_i32(3, 10);
        let target = rng.range_i32(1, count - 2);
        p.assert_same(1, count, target, rng.spicy_i32());
    }
}

#[test]
fn row05_mode1_param1_zero_defaults_to_5() {
    let p = pair();
    // count defaults to 5 -> ids 100..104.
    for b in -8..=8 {
        p.assert_same(1, 0, b, 0);
    }
    let mut rng = Rng::new(0x0105);
    for _ in 0..5_000 {
        p.assert_same(1, 0, rng.range_i32(-8, 8), rng.spicy_i32());
    }
}

#[test]
fn row06_mode1_param1_negative_defaults_to_5() {
    let p = pair();
    for a in [-1, -2, -5, -100, -0x2000_0000, i32::MIN + 1, i32::MIN] {
        for b in -8..=8 {
            p.assert_same(1, a, b, 0);
        }
    }
    let mut rng = Rng::new(0x0106);
    for _ in 0..5_000 {
        let a = rng.range_i32(i32::MIN, -1);
        p.assert_same(1, a, rng.range_i32(-8, 8), rng.spicy_i32());
    }
}

#[test]
fn row07_mode1_large_count() {
    let p = pair();
    let mut rng = Rng::new(0x0107);
    // 4..7 digit `Entry_%d` names, deep pointer walk. 200_000 * 40 == 8 MiB.
    for _ in 0..300 {
        let count = rng.range_i32(1_000, 200_000);
        // in-range target
        p.assert_same(1, count, rng.range_i32(0, count - 1), rng.spicy_i32());
        // out-of-range target -> full walk then -2
        p.assert_same(1, count, count + rng.range_i32(0, 1_000), 0);
        p.assert_same(1, count, -rng.range_i32(1, 1_000), 0);
    }
}

#[test]
fn row08_mode1_param2_full_range() {
    let p = pair();
    let mut rng = Rng::new(0x0108);
    for _ in 0..20_000 {
        let count = rng.range_i32(1, 12);
        p.assert_same(1, count, rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row09_mode1_param3_is_ignored() {
    let p = pair();
    let mut rng = Rng::new(0x0109);
    for _ in 0..5_000 {
        let count = rng.range_i32(1, 10);
        let target = rng.range_i32(-2, count + 2);
        let base = p.assert_same(1, count, target, 0);
        for _ in 0..4 {
            let d = rng.spicy_i32();
            let v = p.assert_same(1, count, target, d);
            assert_eq!(v, base, "mode 1 must ignore param3 (d={d})");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-18: mode 2 — create_entries(base_id=200) + modify_entries + param3
// ---------------------------------------------------------------------------

#[test]
fn row10_mode2_single_identity() {
    let p = pair();
    let r = p.assert_same(2, 1, 1, 0);
    assert_eq!(r, 2000, "sanity: entry 200 has value 2000, multiplier 1");
}

#[test]
fn row11_mode2_identity_multiplier() {
    let p = pair();
    let mut rng = Rng::new(0x0201);
    for _ in 0..5_000 {
        p.assert_same(2, rng.range_i32(1, 10), 1, rng.spicy_i32());
    }
}

#[test]
fn row12_mode2_zero_multiplier_skips_param3() {
    let p = pair();
    let mut rng = Rng::new(0x0202);
    for _ in 0..5_000 {
        let count = rng.range_i32(1, 10);
        let d = rng.spicy_i32();
        let v = p.assert_same(2, count, 0, d);
        assert_eq!(v, 0, "multiplier 0 -> total 0 -> param3 not added");
    }
}

#[test]
fn row13_mode2_small_negative_multiplier() {
    let p = pair();
    let mut rng = Rng::new(0x0203);
    for _ in 0..5_000 {
        p.assert_same(2, rng.range_i32(1, 10), rng.range_i32(-1_000, -1), rng.spicy_i32());
    }
}

#[test]
fn row14_mode2_param1_zero_defaults_to_3() {
    let p = pair();
    let mut rng = Rng::new(0x0204);
    for _ in 0..5_000 {
        p.assert_same(2, 0, rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row15_mode2_param1_negative_defaults_to_3() {
    let p = pair();
    let mut rng = Rng::new(0x0205);
    for a in [-1, -3, -7, -0x4000_0000, i32::MIN + 1, i32::MIN] {
        for _ in 0..500 {
            p.assert_same(2, a, rng.spicy_i32(), rng.spicy_i32());
        }
    }
    for _ in 0..5_000 {
        let a = rng.range_i32(i32::MIN, -1);
        p.assert_same(2, a, rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row16_mode2_wrapping_multiplier() {
    let p = pair();
    let mut rng = Rng::new(0x0206);
    for _ in 0..20_000 {
        p.assert_same(2, rng.range_i32(1, 12), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row17_mode2_large_count_wrapping_total() {
    let p = pair();
    let mut rng = Rng::new(0x0207);
    for _ in 0..400 {
        let count = rng.range_i32(1_000, 200_000);
        p.assert_same(2, count, rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row18_mode2_param3_extremes() {
    let p = pair();
    let mut rng = Rng::new(0x0208);
    for d in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, 1, -1] {
        for _ in 0..1_000 {
            p.assert_same(2, rng.range_i32(1, 10), rng.spicy_i32(), d);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19-21: mode 3 — calculate_lookup over the 4x3 table
// ---------------------------------------------------------------------------

#[test]
fn row19_mode3_all_valid_cells() {
    let p = pair();
    let expect = [
        [10, 20, 30],
        [40, 50, 60],
        [70, 80, 90],
        [100, 110, 120],
    ];
    for row in 0..4i32 {
        for col in 0..3i32 {
            let v = p.assert_same(3, row, col, 0);
            assert_eq!(
                v,
                expect[row as usize][col as usize] * 2,
                "lookup_table[{row}][{col}] * 2"
            );
        }
    }
}

#[test]
fn row20_mode3_all_cells_x_random_param3() {
    let p = pair();
    let mut rng = Rng::new(0x0301);
    for _ in 0..20_000 {
        p.assert_same(3, rng.range_i32(0, 3), rng.range_i32(0, 2), rng.spicy_i32());
    }
    // exhaustive cell x extreme param3
    for row in 0..4i32 {
        for col in 0..3i32 {
            for d in [i32::MAX, i32::MIN, i32::MAX - 200, i32::MIN + 200, 0, -240] {
                p.assert_same(3, row, col, d);
            }
        }
    }
}

#[test]
fn row21_mode3_table_corners() {
    let p = pair();
    let mut rng = Rng::new(0x0302);
    for (row, col) in [(0, 0), (0, 2), (3, 0), (3, 2)] {
        for _ in 0..2_000 {
            p.assert_same(3, row, col, rng.spicy_i32());
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 22-26: default arm — process_name("Default" -> "TestName")
// ---------------------------------------------------------------------------

#[test]
fn row22_default_mode_zero() {
    let p = pair();
    let v = p.assert_same(0, 1, 0, 0);
    assert_eq!(v, 8, "strlen(\"TestName\") * 1");
    let mut rng = Rng::new(0x0401);
    for _ in 0..5_000 {
        p.assert_same(0, rng.range_i32(-1_000, 1_000), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row23_default_mode_four() {
    let p = pair();
    let mut rng = Rng::new(0x0402);
    for _ in 0..5_000 {
        p.assert_same(4, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row24_default_mode_out_of_range() {
    let p = pair();
    let mut rng = Rng::new(0x0403);
    for m in [
        -1,
        -2,
        -3,
        5,
        6,
        100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x2000_0000,
    ] {
        for _ in 0..500 {
            p.assert_same(m, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        }
    }
    for _ in 0..10_000 {
        let m = rng.next_i32();
        if (1..=3).contains(&m) {
            continue;
        }
        p.assert_same(m, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row25_default_wrapping_multiply() {
    let p = pair();
    let mut rng = Rng::new(0x0404);
    for a in [
        i32::MAX,
        i32::MIN,
        0x2000_0000,
        -0x2000_0000,
        0x1000_0000,
        268_435_456,
        i32::MAX / 8,
        i32::MAX / 8 + 1,
    ] {
        p.assert_same(0, a, 0, 0);
    }
    for _ in 0..20_000 {
        p.assert_same(0, rng.next_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn row26_default_ignores_param2_param3() {
    let p = pair();
    let mut rng = Rng::new(0x0405);
    for _ in 0..5_000 {
        let a = rng.next_i32();
        let base = p.assert_same(0, a, 0, 0);
        for _ in 0..3 {
            let v = p.assert_same(0, a, rng.spicy_i32(), rng.spicy_i32());
            assert_eq!(v, base, "default arm must ignore param2/param3");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-30: cross-product fuzz, statelessness, digit-boundary shapes
// ---------------------------------------------------------------------------

#[test]
fn row27_full_cross_product_fuzz() {
    let p = pair();
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..100_000 {
        // Bias `mode` so all four arms are hit often, but also allow any int.
        let m = match rng.next_u64() % 8 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => rng.range_i32(-4, 7),
            _ => rng.spicy_i32(),
        };
        // Keep counts sane for modes 1/2 so we do not try to allocate 80 GiB.
        let (a, b, d) = if m == 1 || m == 2 {
            (rng.range_i32(-20, 64), rng.spicy_i32(), rng.spicy_i32())
        } else {
            (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32())
        };
        p.assert_same(m, a, b, d);
    }
}

#[test]
fn row28_stateless_across_repeated_calls() {
    let p = pair();
    let mut rng = Rng::new(0x0501);
    // Same inputs repeated interleaved with others must give the same answer:
    // neither library may carry state between calls.
    let fixtures: Vec<(i32, i32, i32, i32)> = (0..40)
        .map(|_| {
            let m = rng.range_i32(0, 4);
            (m, rng.range_i32(-4, 12), rng.range_i32(-4, 12), rng.spicy_i32())
        })
        .collect();
    let firsts: Vec<i32> = fixtures
        .iter()
        .map(|&(m, a, b, d)| p.assert_same(m, a, b, d))
        .collect();
    for _round in 0..50 {
        for (i, &(m, a, b, d)) in fixtures.iter().enumerate() {
            // noise between the repeats
            p.assert_same(rng.range_i32(0, 4), rng.range_i32(-4, 12), rng.spicy_i32(), rng.spicy_i32());
            let v = p.assert_same(m, a, b, d);
            assert_eq!(v, firsts[i], "state leaked across calls for ({m},{a},{b},{d})");
        }
    }
}

#[test]
fn row29_mode1_digit_count_boundaries() {
    let p = pair();
    // base_id 100: `Entry_%d` grows 3->4 digits at i=900, 4->5 at i=9900, ...
    let mut rng = Rng::new(0x0601);
    for a in [
        899, 900, 901, 902, 9_899, 9_900, 9_901, 99_899, 99_900, 99_901,
    ] {
        p.assert_same(1, a, 0, 0);
        p.assert_same(1, a, a - 1, 0);
        p.assert_same(1, a, a, 0); // one past -> -2
        for _ in 0..20 {
            p.assert_same(1, a, rng.range_i32(-5, a + 5), rng.spicy_i32());
        }
    }
}

#[test]
fn row30_mode2_digit_count_boundaries() {
    let p = pair();
    // base_id 200: 3->4 digits at i=800, 4->5 at i=9800, ...
    let mut rng = Rng::new(0x0602);
    for a in [
        799, 800, 801, 802, 9_799, 9_800, 9_801, 99_799, 99_800, 99_801,
    ] {
        p.assert_same(2, a, 1, 0);
        for _ in 0..20 {
            p.assert_same(2, a, rng.spicy_i32(), rng.spicy_i32());
        }
    }
}
