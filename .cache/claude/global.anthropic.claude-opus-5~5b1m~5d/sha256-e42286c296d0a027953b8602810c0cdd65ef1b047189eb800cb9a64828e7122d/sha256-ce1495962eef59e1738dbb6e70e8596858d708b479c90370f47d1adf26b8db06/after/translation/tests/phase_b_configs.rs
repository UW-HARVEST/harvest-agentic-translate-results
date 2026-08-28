// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every call goes through BOTH `.so`s via
// libloading; results must be byte-identical. Randomized rows use a fixed
// SplitMix64 seed so failures reproduce exactly.

mod common;
use common::*;

const N: usize = 2000; // randomized cases per row

// ===========================================================================
// mode 1 -- create_entries + find_entry + strcpy(buffer, found->name)
// ===========================================================================

/// Row 1: `param1 <= 0` (count defaults to 5), `param2 = 0` -> first element.
#[test]
fn row01_mode1_default_count_first_element() {
    let mut rng = Rng::new(0x1001);
    for _ in 0..N {
        // every param1 <= 0 must take the `: 5` branch
        let p1 = if rng.next_u64() % 2 == 0 { 0 } else { rng.in_range(i32::MIN, 0) };
        let p3 = rng.edgy_i32();
        let got = same(1, p1, 0, p3);
        assert_eq!(got, 1000, "count-5 default, first id 100 -> value 1000");
    }
}

/// Row 2: `param1 <= 0` (count 5), `param2 = 4` -> last element.
#[test]
fn row02_mode1_default_count_last_element() {
    let mut rng = Rng::new(0x1002);
    for _ in 0..N {
        let p1 = rng.in_range(i32::MIN, 0);
        let p3 = rng.edgy_i32();
        let got = same(1, p1, 4, p3);
        assert_eq!(got, 1040, "count-5 default, last id 104 -> value 1040");
    }
}

/// Row 3: `param1 <= 0` (count 5), interior `param2` in `1..=3`.
#[test]
fn row03_mode1_default_count_interior() {
    let mut rng = Rng::new(0x1003);
    for _ in 0..N {
        let p1 = rng.in_range(i32::MIN, 0);
        let p2 = rng.in_range(1, 3);
        let p3 = rng.edgy_i32();
        let got = same(1, p1, p2, p3);
        assert_eq!(got, (100 + p2) * 10);
    }
}

/// Row 4: single-element array (`param1 == 1`), `param2 == 0`.
#[test]
fn row04_mode1_single_element() {
    let mut rng = Rng::new(0x1004);
    for _ in 0..N {
        let p3 = rng.edgy_i32();
        let got = same(1, 1, 0, p3);
        assert_eq!(got, 1000);
    }
}

/// Row 5: small counts `2..=10`, in-range `param2`.
#[test]
fn row05_mode1_small_counts() {
    let mut rng = Rng::new(0x1005);
    for _ in 0..N {
        let count = rng.in_range(2, 10);
        let p2 = rng.in_range(0, count - 1);
        let p3 = rng.edgy_i32();
        let got = same(1, count, p2, p3);
        assert_eq!(got, (100 + p2) * 10);
    }
}

/// Row 6: counts above the dead `MAX_ENTRIES` constant (11..=64) are accepted.
#[test]
fn row06_mode1_counts_above_max_entries() {
    let mut rng = Rng::new(0x1006);
    assert_eq!(MAX_ENTRIES, 10);
    for _ in 0..N {
        let count = rng.in_range(MAX_ENTRIES + 1, 64);
        let p2 = rng.in_range(0, count - 1);
        let p3 = rng.edgy_i32();
        let got = same(1, count, p2, p3);
        assert_eq!(got, (100 + p2) * 10, "MAX_ENTRIES must NOT be enforced");
    }
}

/// Row 7: large counts (multi-page alloc; ids reach 4 digits so the
/// `sprintf("Entry_%d")` path renders longer strings).
#[test]
fn row07_mode1_large_counts() {
    let mut rng = Rng::new(0x1007);
    for _ in 0..300 {
        let count = rng.in_range(256, SANE_COUNT_MAX);
        let p2 = rng.in_range(0, count - 1);
        let p3 = rng.edgy_i32();
        let got = same(1, count, p2, p3);
        assert_eq!(got, (100 + p2) * 10);
    }
}

/// Row 7b: medium-large counts where `malloc` still SUCCEEDS (up to ~168 MiB),
/// so ids reach 7 digits and `sprintf("Entry_%d")` renders its longest strings
/// on the success path.
#[test]
fn row07b_mode1_medium_large_counts() {
    for count in [1 << 16, 1 << 18, 1 << 20, (1 << 22) + 7] {
        for p2 in [0, 1, count / 2, count - 2, count - 1] {
            let got = same(1, count, p2, 0);
            assert_eq!(got, (100i32.wrapping_add(p2)).wrapping_mul(10), "count={count} p2={p2}");
        }
        // just past the end still rejects
        same_eq(1, count, count, 0, -2);
    }
}

/// Row 20b: mode 2 with medium-large counts -- long wrapping accumulation.
#[test]
fn row20b_mode2_medium_large_counts() {
    for count in [1 << 16, 1 << 18, 1 << 20] {
        for p2 in [1, -1, 3, 7, i32::MAX, i32::MIN, 1 << 16] {
            for p3 in [0, 1, i32::MAX, i32::MIN] {
                let got = same(2, count, p2, p3);
                assert_eq!(got, model_mode2(count, p2, p3), "count={count} p2={p2} p3={p3}");
            }
        }
    }
    // a couple of very large (but still allocatable) counts
    for count in [(1 << 22) + 7, 1 << 23] {
        for p2 in [1, -1, i32::MAX] {
            let got = same(2, count, p2, 12345);
            assert_eq!(got, model_mode2(count, p2, 12345), "count={count} p2={p2}");
        }
    }
}

/// Row 8: exhaustive index x size grid -- every `param2` in `-2..=count+1`
/// for every `count` in `1..=24`.
#[test]
fn row08_mode1_exhaustive_index_size_grid() {
    for count in 1..=24i32 {
        for p2 in -2..=(count + 1) {
            let got = same(1, count, p2, 0);
            let expect = if p2 >= 0 && p2 < count { (100 + p2) * 10 } else { -2 };
            assert_eq!(got, expect, "count={count} p2={p2}");
        }
    }
}

/// Row 9: `param3` is unused in mode 1 -- sweeping it must not change anything.
#[test]
fn row09_mode1_param3_unused() {
    let p3s = [0, 1, -1, 7, -7, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for count in [1, 2, 5, 10, 11, 33] {
        for p2 in 0..count {
            let mut base: Option<i32> = None;
            for &p3 in &p3s {
                let got = same(1, count, p2, p3);
                match base {
                    None => base = Some(got),
                    Some(b) => assert_eq!(b, got, "param3={p3} changed mode-1 result"),
                }
            }
        }
    }
}

/// Row 10: randomized over the full `int` range (mixes found / not-found).
#[test]
fn row10_mode1_randomized_full_range() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..N * 5 {
        let p1 = sane_param1(rng.edgy_i32());
        let p2 = rng.edgy_i32();
        let p3 = rng.edgy_i32();
        same(1, p1, p2, p3);
    }
}

// ===========================================================================
// mode 2 -- create_entries + modify_entries
// ===========================================================================

/// Reference model for mode 2, using the same wrapping arithmetic as the C.
fn model_mode2(p1: i32, p2: i32, p3: i32) -> i32 {
    let count = if p1 > 0 { p1 } else { 3 };
    let mut total: i32 = 0;
    for i in 0..count {
        let value = 200i32.wrapping_add(i).wrapping_mul(10);
        if value != 0 {
            total = total.wrapping_add(value.wrapping_mul(p2));
        }
    }
    if total != 0 { total.wrapping_add(p3) } else { 0 }
}

/// Row 15: `param1 <= 0` (count 3), identity multiplier, `param3 == 0`.
#[test]
fn row15_mode2_default_count_identity() {
    let mut rng = Rng::new(0x2015);
    for _ in 0..N {
        let p1 = rng.in_range(i32::MIN, 0);
        let got = same(2, p1, 1, 0);
        assert_eq!(got, 2000 + 2010 + 2020);
        assert_eq!(got, model_mode2(p1, 1, 0));
    }
}

/// Row 16: `param1 <= 0` (count 3), multiplier `-1`, randomized `param3`.
#[test]
fn row16_mode2_default_count_negate() {
    let mut rng = Rng::new(0x2016);
    for _ in 0..N {
        let p1 = rng.in_range(i32::MIN, 0);
        let p3 = rng.edgy_i32();
        let got = same(2, p1, -1, p3);
        assert_eq!(got, model_mode2(p1, -1, p3));
    }
}

/// Row 17: single element, randomized non-zero multiplier + addend.
#[test]
fn row17_mode2_single_element() {
    let mut rng = Rng::new(0x2017);
    for _ in 0..N {
        let mut p2 = rng.any_i32();
        if p2 == 0 {
            p2 = 1;
        }
        let p3 = rng.edgy_i32();
        let got = same(2, 1, p2, p3);
        assert_eq!(got, model_mode2(1, p2, p3));
    }
}

/// Row 18: small counts `2..=10`, randomized non-zero multiplier + addend.
#[test]
fn row18_mode2_small_counts() {
    let mut rng = Rng::new(0x2018);
    for _ in 0..N {
        let count = rng.in_range(2, 10);
        let mut p2 = rng.any_i32();
        if p2 == 0 {
            p2 = 3;
        }
        let p3 = rng.edgy_i32();
        let got = same(2, count, p2, p3);
        assert_eq!(got, model_mode2(count, p2, p3));
    }
}

/// Row 19: counts above the dead `MAX_ENTRIES`.
#[test]
fn row19_mode2_counts_above_max_entries() {
    let mut rng = Rng::new(0x2019);
    for _ in 0..N {
        let count = rng.in_range(MAX_ENTRIES + 1, 64);
        let p2 = rng.edgy_i32();
        let p3 = rng.edgy_i32();
        let got = same(2, count, p2, p3);
        assert_eq!(got, model_mode2(count, p2, p3));
    }
}

/// Row 20: large counts -- `total` accumulates through many signed wraps.
#[test]
fn row20_mode2_large_counts_wrapping_total() {
    let mut rng = Rng::new(0x2020);
    for _ in 0..300 {
        let count = rng.in_range(256, SANE_COUNT_MAX);
        let p2 = rng.edgy_i32();
        let p3 = rng.edgy_i32();
        let got = same(2, count, p2, p3);
        assert_eq!(got, model_mode2(count, p2, p3));
    }
}

/// Row 21: multiplier at extremes -- per-element `value * multiplier` wraps.
#[test]
fn row21_mode2_multiplier_extremes() {
    let mults = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 16,
        -(1 << 16),
        1 << 24,
        -(1 << 24),
        1 << 30,
        -(1 << 30),
        0x5555_5555,
        -0x5555_5555,
    ];
    for &p2 in &mults {
        for count in [-1, 0, 1, 2, 3, 7, 10, 11, 64, 257] {
            for p3 in [0, 1, -1, i32::MAX, i32::MIN] {
                let got = same(2, count, p2, p3);
                assert_eq!(got, model_mode2(count, p2, p3), "count={count} p2={p2} p3={p3}");
            }
        }
    }
}

/// Row 22: addend at extremes -- wrapping `result += param3`.
#[test]
fn row22_mode2_param3_extremes() {
    let mut rng = Rng::new(0x2022);
    for &p3 in &[i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, 1, -1] {
        for _ in 0..200 {
            let count = sane_param1(rng.edgy_i32());
            let p2 = rng.edgy_i32();
            let got = same(2, count, p2, p3);
            assert_eq!(got, model_mode2(count, p2, p3));
        }
    }
}

// ===========================================================================
// mode 3 -- lookup_table + calculate_lookup
// ===========================================================================

/// Row 23: exhaustive in-range 4x3 grid, `param3 == 0`.
#[test]
fn row23_mode3_full_grid() {
    for row in 0..4i32 {
        for col in 0..3i32 {
            let got = same(3, row, col, 0);
            assert_eq!(got, LOOKUP_TABLE[row as usize][col as usize] * 2, "cell ({row},{col})");
        }
    }
}

/// Row 24: full grid x randomized `param3` (wrapping add).
#[test]
fn row24_mode3_grid_randomized_param3() {
    let mut rng = Rng::new(0x3024);
    for _ in 0..N {
        let row = rng.in_range(0, 3);
        let col = rng.in_range(0, 2);
        let p3 = rng.any_i32();
        let got = same(3, row, col, p3);
        let expect = (LOOKUP_TABLE[row as usize][col as usize] * 2).wrapping_add(p3);
        assert_eq!(got, expect);
    }
}

/// Row 25: full grid x boundary addends.
#[test]
fn row25_mode3_grid_boundary_param3() {
    for row in 0..4i32 {
        for col in 0..3i32 {
            for &p3 in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
                let got = same(3, row, col, p3);
                let expect = (LOOKUP_TABLE[row as usize][col as usize] * 2).wrapping_add(p3);
                assert_eq!(got, expect, "cell ({row},{col}) p3={p3}");
            }
        }
    }
}

/// Row 26: the four in-range corners x extreme addends.
#[test]
fn row26_mode3_corners() {
    for &(row, col) in &[(0i32, 0i32), (0, 2), (3, 0), (3, 2)] {
        for &p3 in &[i32::MIN, -1, 0, 1, i32::MAX] {
            let got = same(3, row, col, p3);
            let expect = (LOOKUP_TABLE[row as usize][col as usize] * 2).wrapping_add(p3);
            assert_eq!(got, expect);
        }
    }
}

// ===========================================================================
// default arm -- process_name + strcpy + strlen
// ===========================================================================

/// Row 27: `mode == 0`, randomized `param1` -> `8 * param1`.
#[test]
fn row27_default_mode_zero() {
    let mut rng = Rng::new(0x4027);
    for _ in 0..N {
        let p1 = rng.any_i32();
        let got = same(0, p1, rng.any_i32(), rng.any_i32());
        assert_eq!(got, 8i32.wrapping_mul(p1), "strlen(\"TestName\") == 8");
    }
}

/// Row 28: randomized `mode` outside `{1,2,3}`.
#[test]
fn row28_default_randomized_mode() {
    let mut rng = Rng::new(0x4028);
    let mut n = 0usize;
    while n < N * 2 {
        let mode = rng.edgy_i32();
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        let p1 = rng.edgy_i32();
        let got = same(mode, p1, rng.any_i32(), rng.any_i32());
        assert_eq!(got, 8i32.wrapping_mul(p1), "mode={mode}");
        n += 1;
    }
}

/// Row 29: `param2` / `param3` are unused in the default arm.
#[test]
fn row29_default_param2_param3_unused() {
    let vals = [0, 1, -1, 5, -5, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    for &mode in &[0, -1, 4, 5, 100, -100, i32::MAX, i32::MIN] {
        for &p1 in &[0, 1, -1, 7, i32::MAX, i32::MIN] {
            let mut base: Option<i32> = None;
            for &p2 in &vals {
                for &p3 in &vals {
                    let got = same(mode, p1, p2, p3);
                    match base {
                        None => base = Some(got),
                        Some(b) => assert_eq!(b, got, "p2={p2}/p3={p3} changed default-arm result"),
                    }
                }
            }
        }
    }
}

/// Row 30: `8 * param1` wraps.
#[test]
fn row30_default_param1_overflow() {
    for &p1 in &[
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1 << 28,
        3 << 28,
        1 << 29,
        268_435_456,
        -268_435_456,
        0x1FFF_FFFF,
    ] {
        let got = same(0, p1, 0, 0);
        assert_eq!(got, 8i32.wrapping_mul(p1), "p1={p1}");
    }
}

// ===========================================================================
// cross-mode
// ===========================================================================

/// Row 31: mode swept over `-8..=8` (covers all switch arms + neighbours).
#[test]
fn row31_mode_sweep() {
    let mut rng = Rng::new(0x5031);
    for mode in -8..=8i32 {
        for _ in 0..400 {
            let p1 = sane_param1(rng.edgy_i32());
            let p2 = rng.edgy_i32();
            let p3 = rng.edgy_i32();
            same(mode, p1, p2, p3);
        }
    }
}

/// Row 32: fully randomized fuzz over all four arguments.
#[test]
fn row32_full_fuzz() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..200_000 {
        let mode = match rng.next_u64() % 8 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 0,
            _ => rng.edgy_i32(),
        };
        let raw1 = rng.edgy_i32();
        // Keep mode-1/2 allocations cheap; other modes ignore the magnitude.
        let p1 = if mode == 1 || mode == 2 { sane_param1(raw1) } else { raw1 };
        let p2 = rng.edgy_i32();
        let p3 = rng.edgy_i32();
        same(mode, p1, p2, p3);
    }
}
