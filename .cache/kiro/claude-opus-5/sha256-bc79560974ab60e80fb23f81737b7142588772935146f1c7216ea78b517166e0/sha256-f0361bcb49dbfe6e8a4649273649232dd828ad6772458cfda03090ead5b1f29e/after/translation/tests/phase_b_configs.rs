//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are reached only through the `dataentry` dynamic symbol
//! of their respective shared objects.

mod harness;

use harness::{Rng, assert_same};
use std::ffi::c_int;

const IMAX: c_int = c_int::MAX;
const IMIN: c_int = c_int::MIN;

/// Row 1 — mode 1, small positive count, index hits.
#[test]
fn row01_mode1_small_counts_hit() {
    for p1 in 1..=3 {
        for p2 in 0..p1 {
            assert_same("row01", 1, p1, p2, 0);
        }
    }
}

/// Row 2 — mode 1, randomized count in [1,512] with a hitting index.
#[test]
fn row02_mode1_randomized_hit() {
    let mut rng = Rng::new(0xB0002);
    for _ in 0..20_000 {
        let p1 = rng.range(1, 512);
        let p2 = rng.range(0, (p1 - 1) as i64);
        let p3 = rng.i32();
        assert_same("row02", 1, p1, p2, p3);
    }
}

/// Row 3 — mode 1, `param1 <= 0` so count defaults to 5; index hits.
#[test]
fn row03_mode1_default_count_hit() {
    let mut rng = Rng::new(0xB0003);
    for p2 in 0..5 {
        for p1 in [0, -1, -5, -100, IMIN] {
            assert_same("row03", 1, p1, p2, 0);
        }
    }
    for _ in 0..5_000 {
        let p1 = rng.range(IMIN as i64, 0);
        let p2 = rng.range(0, 4);
        assert_same("row03", 1, p1, p2, rng.i32());
    }
}

/// Row 4 — mode 1, default count 5, index misses.
#[test]
fn row04_mode1_default_count_miss() {
    for p1 in [0, -1, -7, IMIN] {
        for p2 in [5, 6, 100, -1, -2, IMIN, IMAX] {
            assert_same("row04", 1, p1, p2, 0);
        }
    }
}

/// Row 5 — mode 1, exact boundary of the found range (`param2 == count` and
/// `param2 == count - 1`).
#[test]
fn row05_mode1_index_boundary() {
    let mut rng = Rng::new(0xB0005);
    for _ in 0..5_000 {
        let p1 = rng.range(1, 512);
        let p3 = rng.i32();
        assert_same("row05", 1, p1, p1 - 1, p3);
        assert_same("row05", 1, p1, p1, p3);
        assert_same("row05", 1, p1, p1 + 1, p3);
        assert_same("row05", 1, p1, -1, p3);
    }
}

/// Row 6 — mode 1, "one" shape: a single entry.
#[test]
fn row06_mode1_single_entry() {
    assert_same("row06", 1, 1, 0, 0);
    assert_same("row06", 1, 1, 0, IMAX);
    assert_same("row06", 1, 1, 1, 0);
    assert_same("row06", 1, 1, -1, 0);
}

/// Row 7 — mode 1, large-but-allocatable counts; wide `sprintf` ids.
#[test]
fn row07_mode1_large_counts() {
    for p1 in [1_000, 65_536, 200_000] {
        for p2 in [0, 1, 9, 99, 999, p1 / 2, p1 - 1, p1] {
            assert_same("row07", 1, p1, p2, 0);
        }
    }
}

/// Row 8 — mode 1, `param3` is ignored: vary it while the rest is fixed.
#[test]
fn row08_mode1_param3_ignored() {
    let mut rng = Rng::new(0xB0008);
    for _ in 0..10_000 {
        let p3 = rng.i32();
        assert_same("row08", 1, 7, 3, p3);
        assert_same("row08", 1, 0, 4, p3);
        assert_same("row08", 1, -3, 9, p3);
    }
}

/// Row 9 — mode 2, `param1 <= 0` so count defaults to 3.
#[test]
fn row09_mode2_default_count() {
    let mut rng = Rng::new(0xB0009);
    for _ in 0..20_000 {
        let p1 = rng.range(IMIN as i64, 0);
        let mut p2 = rng.i32();
        if p2 == 0 {
            p2 = 1;
        }
        let p3 = rng.i32();
        assert_same("row09", 2, p1, p2, p3);
    }
}

/// Row 10 — mode 2, randomized explicit count in [1,512].
#[test]
fn row10_mode2_randomized_counts() {
    let mut rng = Rng::new(0xB0010);
    for _ in 0..20_000 {
        let p1 = rng.range(1, 512);
        let mut p2 = rng.i32();
        if p2 == 0 {
            p2 = -1;
        }
        let p3 = rng.i32();
        assert_same("row10", 2, p1, p2, p3);
    }
}

/// Row 11 — mode 2 with `param2 == 0`: total collapses to 0 and `param3` is NOT
/// added.
#[test]
fn row11_mode2_zero_multiplier() {
    let mut rng = Rng::new(0xB0011);
    for p1 in [-1, 0, 1, 2, 3, 5, 64, 512] {
        for p3 in [0, 1, -1, IMAX, IMIN, 12345] {
            assert_same("row11", 2, p1, 0, p3);
        }
    }
    for _ in 0..10_000 {
        let p1 = rng.range(-16, 512);
        let p3 = rng.i32();
        assert_same("row11", 2, p1, 0, p3);
    }
}

/// Row 12 — mode 2 with negative multipliers.
#[test]
fn row12_mode2_negative_multiplier() {
    let mut rng = Rng::new(0xB0012);
    for _ in 0..20_000 {
        let p1 = rng.range(1, 256);
        let p2 = rng.range(IMIN as i64, -1);
        let p3 = rng.i32();
        assert_same("row12", 2, p1, p2, p3);
    }
}

/// Row 13 — mode 2 where the accumulated total overflows `int`.
#[test]
fn row13_mode2_overflowing_total() {
    for p1 in [1, 3, 64, 512] {
        for p2 in [IMAX, IMIN, 0x10001, -0x10001, 1 << 20, -(1 << 20), 0x7FFF_FFFE] {
            for p3 in [0, 1, -1, IMAX, IMIN] {
                assert_same("row13", 2, p1, p2, p3);
            }
        }
    }
}

/// Row 14 — mode 2 with "one" and "many" shapes.
#[test]
fn row14_mode2_one_and_many() {
    for p1 in [1, 1_000, 65_536] {
        for p2 in [1, 2, -3, 7, IMAX] {
            for p3 in [0, 42, -42, IMIN] {
                assert_same("row14", 2, p1, p2, p3);
            }
        }
    }
}

/// Row 15 — mode 3, all 12 valid lookup-table cells with `param3 == 0`.
#[test]
fn row15_mode3_all_cells() {
    for row in 0..4 {
        for col in 0..3 {
            assert_same("row15", 3, row, col, 0);
        }
    }
}

/// Row 16 — mode 3, all valid cells x randomized `param3`.
#[test]
fn row16_mode3_cells_randomized_param3() {
    let mut rng = Rng::new(0xB0016);
    for _ in 0..20_000 {
        let row = rng.range(0, 3);
        let col = rng.range(0, 2);
        let p3 = rng.i32();
        assert_same("row16", 3, row, col, p3);
    }
}

/// Row 17 — mode 3, addition boundaries.
#[test]
fn row17_mode3_addition_boundaries() {
    for row in 0..4 {
        for col in 0..3 {
            for p3 in [IMAX, IMIN, -1, 0, 1, IMAX - 100, IMIN + 100] {
                assert_same("row17", 3, row, col, p3);
            }
        }
    }
}

/// Row 18 — default arm reached through a spread of non-1/2/3 `mode` values.
#[test]
fn row18_default_mode_values() {
    for mode in [0, 4, 5, 6, 7, 100, -1, -7, IMIN, IMAX] {
        assert_same("row18", mode, 1, 0, 0);
    }
}

/// Row 19 — default arm, randomized `mode` x randomized `param1`.
#[test]
fn row19_default_randomized() {
    let mut rng = Rng::new(0xB0019);
    let mut done = 0;
    while done < 20_000 {
        let mode = rng.i32();
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        let p1 = rng.i32();
        let p2 = rng.i32();
        let p3 = rng.i32();
        assert_same("row19", mode, p1, p2, p3);
        done += 1;
    }
}

/// Row 20 — default arm, multiplier boundaries and overflow.
#[test]
fn row20_default_multiplier_boundaries() {
    for mode in [0, -1, 9, IMIN, IMAX] {
        for p1 in [0, 1, -1, IMAX, IMIN, 0x1000_0000, 0x2000_0000, -0x1000_0000] {
            assert_same("row20", mode, p1, 0, 0);
        }
    }
}

/// Row 21 — default arm ignores `param2`/`param3` identically.
#[test]
fn row21_default_ignores_param2_param3() {
    let mut rng = Rng::new(0xB0021);
    for _ in 0..10_000 {
        let p2 = rng.i32();
        let p3 = rng.i32();
        assert_same("row21", 0, 3, p2, p3);
        assert_same("row21", 77, -5, p2, p3);
    }
}

/// Row 22 — fully randomized 4-tuple sweep across all four axes at once.
#[test]
fn row22_randomized_all_axes() {
    let mut rng = Rng::new(0xB0022);
    for _ in 0..200_000 {
        // `mode` biased toward the interesting labels but still unbounded.
        let mode = match rng.range(0, 9) {
            0..=5 => rng.range(-2, 6),
            6..=8 => rng.range(1, 3),
            _ => rng.i32(),
        };
        // `param1` kept in an allocatable band for modes 1/2 while still
        // covering the non-positive side; full-range for the other modes.
        let p1 = if mode == 1 || mode == 2 {
            rng.range(-64, 300)
        } else {
            rng.i32()
        };
        let p2 = match rng.range(0, 3) {
            0 => rng.range(-8, 16),
            1 => rng.range(0, 3),
            _ => rng.i32(),
        };
        let p3 = if rng.range(0, 1) == 0 {
            rng.range(-1000, 1000)
        } else {
            rng.i32()
        };
        assert_same("row22", mode, p1, p2, p3);
    }
}

/// Row 23 — exhaustive small cube over every axis.
#[test]
fn row23_exhaustive_small_cube() {
    for mode in -4..=8 {
        for p1 in -4..=8 {
            for p2 in -4..=8 {
                for p3 in -4..=8 {
                    assert_same("row23", mode, p1, p2, p3);
                }
            }
        }
    }
}
