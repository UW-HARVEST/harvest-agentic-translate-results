//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid
//! input/condition, calls BOTH `.so`s, and asserts they return the SAME
//! sentinel — and that the sentinel is the one the C source actually produces
//! (`-1`, `-2`, or `0`), not merely "both failed somehow".

mod common;

use common::{pair, Rng, EDGE_INTS};

// ---------------------------------------------------------------------------
// Rows 1 & 2 — `create_entries` returns NULL
//   row 1: malloc failed (`count * sizeof(DataEntry)` too large)
//   row 2: `count <= 0` after a successful malloc
// Both surface as -1 from `dataentry`. `count <= 0` is unreachable through the
// public API (both arms coerce via `param1 > 0 ? param1 : 5|3`), so row 2 is
// pinned at the observable level: a non-positive `param1` must NOT yield -1.
// ---------------------------------------------------------------------------
#[test]
fn row01_create_entries_malloc_failure_returns_minus1() {
    let p = pair();
    for mode in [1, 2] {
        for param1 in [i32::MAX, i32::MAX - 1, i32::MAX / 2, 0x4000_0000, 0x2000_0000] {
            p.assert_same_and_eq(mode, param1, 0, 0, -1);
        }
    }
}

#[test]
fn row02_create_entries_nonpositive_count_unreachable() {
    let p = pair();
    for param1 in [0, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        // mode 1 -> count 5 -> index 0 is valid -> 1000, never -1.
        p.assert_same_and_eq(1, param1, 0, 0, 1000);
        // mode 2 -> count 3, multiplier 1 -> 6030, never -1.
        p.assert_same_and_eq(2, param1, 1, 0, 6030);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — `find_entry` with `count <= 0`: loop body never runs -> NULL -> -2.
// Unreachable through `dataentry` (count is always >= 1); asserted at the
// observable level that the count-defaulting path is taken instead.
// ---------------------------------------------------------------------------
#[test]
fn row03_find_entry_nonpositive_count() {
    let p = pair();
    for param1 in [0, -1, -7, i32::MIN] {
        // count defaults to 5, so param2 == 5 is one past the end -> -2.
        p.assert_same_and_eq(1, param1, 5, 0, -2);
        p.assert_same_and_eq(1, param1, 4, 0, 1040);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — `find_entry` exhausts without a match -> NULL -> -2
// ---------------------------------------------------------------------------
#[test]
fn row04_find_entry_no_match() {
    let p = pair();
    let mut rng = Rng::new(0x2004);
    for count in [1, 2, 3, 5, 10, 64] {
        for _ in 0..500 {
            let param2 = rng.spicy_i32();
            let want = if param2 >= 0 && param2 < count {
                (100 + param2) * 10
            } else {
                -2
            };
            p.assert_same_and_eq(1, count, param2, 0, want);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5 & 6 — `process_name` returns -1 (`dest == NULL` / `*dest == '\0'`)
// Dead in the C: the default arm always passes `buffer` holding "Default",
// whose first byte is 'D'. The observable consequence is that the default arm
// never returns -1 but `8 * param1`.
// ---------------------------------------------------------------------------
#[test]
fn row05_06_process_name_guard_is_dead() {
    let p = pair();
    let mut rng = Rng::new(0x2005);
    for mode in [i32::MIN, i32::MIN + 1, -1000, -1, 0, 4, 1000, i32::MAX] {
        p.assert_same_and_eq(mode, 1, 0, 0, 8);
        for _ in 0..500 {
            let param1 = rng.spicy_i32();
            p.assert_same_and_eq(mode, param1, rng.spicy_i32(), rng.spicy_i32(), 8i32.wrapping_mul(param1));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — `calculate_lookup` returns 0 when the table cell is 0.
// Every cell of `lookup_table` is non-zero, so mode 3 with valid indices never
// returns the bare `0`; it always returns `cell * 2 + param3`.
// ---------------------------------------------------------------------------
#[test]
fn row07_calculate_lookup_zero_cell_is_dead() {
    let p = pair();
    let table = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
    for row in 0..4usize {
        for col in 0..3usize {
            let want = (table[row][col] * 2i32).wrapping_add(0);
            p.assert_same_and_eq(3, row as i32, col as i32, 0, want);
            assert_ne!(want, 0, "no lookup cell is zero, so the 0-return is dead");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — `modify_entries` returns -1 on NULL. Guarded by the caller, so mode 2
// never returns -1 unless `create_entries` itself failed (row 1).
// ---------------------------------------------------------------------------
#[test]
fn row08_modify_entries_null_guard_is_dead() {
    let p = pair();
    let mut rng = Rng::new(0x2008);
    for _ in 0..5000 {
        let count = rng.range(-4, 48);
        let got = p.assert_same(2, count, rng.spicy_i32(), rng.spicy_i32());
        assert_ne!(got, -1, "mode 2 with a small count must not report -1");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — mode 1: `entries == NULL || count == 0` -> -1
// ---------------------------------------------------------------------------
#[test]
fn row09_mode1_minus1() {
    let p = pair();
    for param1 in [i32::MAX, 0x4000_0000, 0x3000_0000] {
        for param2 in EDGE_INTS {
            p.assert_same_and_eq(1, param1, param2, 0, -1);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — mode 1: `param2 < 0` -> no match -> -2
// ---------------------------------------------------------------------------
#[test]
fn row10_mode1_negative_index() {
    let p = pair();
    for count in [-1, 0, 1, 2, 3, 5, 10, 100] {
        for param2 in [-1, -2, -10, -100, -1000, i32::MIN + 1, i32::MIN] {
            p.assert_same_and_eq(1, count, param2, 0, -2);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — mode 1: `param2 >= count` (one past the last valid index) -> -2
// ---------------------------------------------------------------------------
#[test]
fn row11_mode1_index_one_past_end() {
    let p = pair();
    for count in [1, 2, 3, 4, 5, 10, 11, 100, 1000] {
        // last valid
        p.assert_same_and_eq(1, count, count - 1, 0, (100 + count - 1) * 10);
        // one past
        p.assert_same_and_eq(1, count, count, 0, -2);
        p.assert_same_and_eq(1, count, count + 1, 0, -2);
    }
    for param1 in [0, -1, i32::MIN] {
        // default count 5: 4 valid, 5 one past
        p.assert_same_and_eq(1, param1, 4, 0, 1040);
        p.assert_same_and_eq(1, param1, 5, 0, -2);
    }
    for param2 in [i32::MAX, i32::MAX - 1, 100_000] {
        p.assert_same_and_eq(1, 10, param2, 0, -2);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — mode 1: `found->id == 0` is dead (ids are `100 + i`)
// ---------------------------------------------------------------------------
#[test]
fn row12_mode1_found_id_zero_is_dead() {
    let p = pair();
    // The only way to target id 0 is param2 == -100, which finds nothing.
    p.assert_same_and_eq(1, 200, -100, 0, -2);
    let mut rng = Rng::new(0x2012);
    for _ in 0..5000 {
        let count = rng.range(1, 64);
        let param2 = rng.range(0, count - 1);
        // A found entry always has a non-zero id, hence a non-zero value.
        p.assert_same_and_eq(1, count, param2, 0, (100 + param2) * 10);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — mode 2: `entries == NULL` -> -1
// ---------------------------------------------------------------------------
#[test]
fn row13_mode2_minus1() {
    let p = pair();
    for param1 in [i32::MAX, i32::MAX - 1, 0x4000_0000, 0x3000_0000] {
        for param2 in [0, 1, -1, i32::MAX, i32::MIN] {
            p.assert_same_and_eq(2, param1, param2, 12345, -1);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — mode 2: total == 0 -> `param3` is NOT added
// ---------------------------------------------------------------------------
#[test]
fn row14_mode2_zero_total_skips_param3() {
    let p = pair();
    let mut rng = Rng::new(0x2014);
    for count in [-5, 0, 1, 2, 3, 4, 10, 33, 250] {
        for param3 in EDGE_INTS {
            p.assert_same_and_eq(2, count, 0, param3, 0);
        }
        for _ in 0..100 {
            p.assert_same_and_eq(2, count, 0, rng.spicy_i32(), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-18 — mode 3 range rejections, each boundary separately
// ---------------------------------------------------------------------------
#[test]
fn row15_mode3_row_below_range() {
    let p = pair();
    for param1 in [-1, -2, -3, -4, -100, -1000, i32::MIN + 1, i32::MIN] {
        for param2 in [-1, 0, 1, 2, 3, i32::MAX, i32::MIN] {
            for param3 in [0, 5, -5, i32::MAX, i32::MIN] {
                p.assert_same_and_eq(3, param1, param2, param3, 0);
            }
        }
    }
}

#[test]
fn row16_mode3_row_at_or_past_range() {
    let p = pair();
    for param1 in [4, 5, 6, 10, 1000, i32::MAX - 1, i32::MAX] {
        for param2 in [-1, 0, 1, 2, 3, i32::MAX, i32::MIN] {
            for param3 in [0, 5, -5, i32::MAX, i32::MIN] {
                p.assert_same_and_eq(3, param1, param2, param3, 0);
            }
        }
    }
    // Exactly one step past the last valid row.
    p.assert_same_and_eq(3, 3, 2, 0, 240);
    p.assert_same_and_eq(3, 4, 2, 0, 0);
}

#[test]
fn row17_mode3_col_below_range() {
    let p = pair();
    for param2 in [-1, -2, -3, -100, i32::MIN + 1, i32::MIN] {
        for param1 in [0, 1, 2, 3] {
            for param3 in [0, 5, -5, i32::MAX, i32::MIN] {
                p.assert_same_and_eq(3, param1, param2, param3, 0);
            }
        }
    }
}

#[test]
fn row18_mode3_col_at_or_past_range() {
    let p = pair();
    for param2 in [3, 4, 5, 100, i32::MAX - 1, i32::MAX] {
        for param1 in [0, 1, 2, 3] {
            for param3 in [0, 5, -5, i32::MAX, i32::MIN] {
                p.assert_same_and_eq(3, param1, param2, param3, 0);
            }
        }
    }
    // Exactly one step past the last valid column.
    p.assert_same_and_eq(3, 3, 2, 0, 240);
    p.assert_same_and_eq(3, 3, 3, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 19 — mode 3 with extreme out-of-range indices
// ---------------------------------------------------------------------------
#[test]
fn row19_mode3_extreme_indices() {
    let p = pair();
    for param1 in EDGE_INTS {
        for param2 in EDGE_INTS {
            let valid = (0..4).contains(&param1) && (0..3).contains(&param2);
            for param3 in [0, 7, -7, i32::MAX, i32::MIN] {
                let got = p.assert_same(3, param1, param2, param3);
                if !valid {
                    assert_eq!(got, 0, "mode 3 out-of-range indices must return 0");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — out-of-range "enum" values for `mode` crossing the FFI boundary.
// A C enum/switch accepts any int; every unmatched value takes `default`.
// ---------------------------------------------------------------------------
#[test]
fn row20_out_of_range_mode_values() {
    let p = pair();
    // Every mode outside {1,2,3} must take the default arm -> 8 * param1.
    let mut modes: Vec<i32> = (-40..=40).collect();
    modes.extend_from_slice(&[
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN / 2,
        -1_000_000,
        1_000_000,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ]);
    for mode in modes {
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        for param1 in [0, 1, -1, 3, -3, 1000, i32::MAX, i32::MIN, i32::MAX - 1] {
            p.assert_same_and_eq(mode, param1, 0, 0, 8i32.wrapping_mul(param1));
        }
    }
    // And a randomized sweep over the whole int domain for `mode`.
    let mut rng = Rng::new(0x2020);
    for _ in 0..500_000 {
        let mode = rng.i32();
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        let param1 = rng.spicy_i32();
        p.assert_same_and_eq(
            mode,
            param1,
            rng.spicy_i32(),
            rng.spicy_i32(),
            8i32.wrapping_mul(param1),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 21 — default arm: `strlen(buffer)` is never 0 (dead branch)
// ---------------------------------------------------------------------------
#[test]
fn row21_default_arm_strlen_never_zero() {
    let p = pair();
    // If the branch were taken, the result would be process_name's 8; since it
    // is always taken, the result is 8 * param1. param1 == 1 makes them equal,
    // so use param1 != 1 to distinguish.
    p.assert_same_and_eq(0, 2, 0, 0, 16);
    p.assert_same_and_eq(0, 0, 0, 0, 0);
    p.assert_same_and_eq(0, -1, 0, 0, -8);
}

// ---------------------------------------------------------------------------
// Row 22 — mode 1, default count 5, param2 outside [0,5) -> -2
// ---------------------------------------------------------------------------
#[test]
fn row22_mode1_default_count_out_of_range_index() {
    let p = pair();
    for param1 in [0, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        for param2 in [-1, -2, 5, 6, 7, 100, i32::MAX, i32::MIN] {
            p.assert_same_and_eq(1, param1, param2, 0, -2);
        }
        for param2 in 0..5 {
            p.assert_same_and_eq(1, param1, param2, 0, (100 + param2) * 10);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — mode 2, default count 3
// ---------------------------------------------------------------------------
#[test]
fn row23_mode2_default_count_three() {
    let p = pair();
    for param1 in [0, -1, -3, -1000, i32::MIN, i32::MIN + 1] {
        for mult in [1, 2, -1, 0, i32::MAX, i32::MIN, 7] {
            let mut want: i32 = 0;
            for i in 0..3i32 {
                want = want.wrapping_add(((200 + i) * 10).wrapping_mul(mult));
            }
            let expect = if want != 0 { want.wrapping_add(99) } else { 0 };
            p.assert_same_and_eq(2, param1, mult, 99, expect);
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep: zero / negative / oversized lengths, and values
// one step past every documented range, for all four parameters at once.
// ---------------------------------------------------------------------------
#[test]
fn generic_boundary_sweep() {
    let p = pair();
    let boundaries = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        10,
        11,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &mode in &boundaries {
        for &a in &boundaries {
            // Keep modes 1/2 from requesting multi-gigabyte allocations
            // repeatedly; the huge values are covered by rows 1/9/13.
            let a = if (mode == 1 || mode == 2) && a > 1_000_000 {
                11
            } else {
                a
            };
            for &b in &boundaries {
                for &c in &[i32::MIN, -1, 0, 1, i32::MAX] {
                    p.assert_same(mode, a, b, c);
                }
            }
        }
    }
}
