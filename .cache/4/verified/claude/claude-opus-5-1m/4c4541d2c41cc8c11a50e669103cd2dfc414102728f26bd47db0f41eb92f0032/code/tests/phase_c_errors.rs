//! Phase C — error-path differential tests, one test per row of ERRORS.md.
//!
//! Each test constructs the exact rejection condition, calls BOTH `.so`s
//! through the exported `dataentry` symbol, and asserts they return the SAME
//! sentinel (`-1`, `-2`, `0`, ...), not merely "both failed".

mod common;

use common::{Pair, Rng, SEED};

// ---------------------------------------------------------------------------
// Rows 1 & 2 — process_name's `dest == NULL || *dest == '\0'` guards (-1)
// ---------------------------------------------------------------------------
// Unreachable by construction through the public ABI: the only call site is
// `process_name(buffer, "TestName", NAME_LENGTH)` in the `default:` branch,
// and `strcpy(buffer, "Default")` runs immediately before it, so `buffer` is
// never NULL and `buffer[0] == 'D' != '\0'`. The guard therefore never fires
// and the branch always ends with `result = strlen("TestName") * param1`.
// This test pins that observable consequence in both implementations.
#[test]
fn err_01_02_process_name_guards() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..2000 {
        let mut mode = rng.mixed_i32();
        if (1..=3).contains(&mode) {
            mode = 0;
        }
        let p1 = rng.mixed_i32();
        let r = p.assert_same("err01_02", mode, p1, rng.mixed_i32(), rng.mixed_i32());
        // If either guard had fired, process_name would have returned -1; the
        // guard result is then overwritten by `strlen(buffer) * param1 == 8*p1`.
        assert_eq!(
            r,
            8i32.wrapping_mul(p1),
            "default branch must yield 8*param1 (guards never fire)"
        );
        assert_eq!(
            r % 8,
            0,
            "result is always a multiple of 8, never the -1 sentinel of process_name"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 3 & 9 — find_entry returns NULL => dataentry mode 1 result -2
// ---------------------------------------------------------------------------
#[test]
fn err_03_09_find_entry_miss() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 103);

    // hand-picked: one past the end, negative, far away, default count
    p.assert_same_and_eq("err03/default-count", 1, 0, 5, 0, -2);
    p.assert_same_and_eq("err03/negative", 1, 0, -1, 0, -2);
    p.assert_same_and_eq("err03/single", 1, 1, 1, 0, -2);
    p.assert_same_and_eq("err03/far", 1, 10, 1_000_000, 0, -2);

    for _ in 0..500 {
        let count = rng.range(1, 128);
        // G3: exactly one step past the last valid index
        p.assert_same_and_eq("err03/one-past", 1, count, count, rng.mixed_i32(), -2);
        // arbitrary out-of-range indices
        let bad = if rng.next_u64() % 2 == 0 {
            rng.range(count, i32::MAX / 4)
        } else {
            rng.range(i32::MIN / 4, -1)
        };
        p.assert_same_and_eq("err03/random-miss", 1, count, bad, rng.mixed_i32(), -2);
    }
}

// ---------------------------------------------------------------------------
// Rows 4, 8, 12 — malloc failure => create_entries NULL => result -1
// ---------------------------------------------------------------------------
#[test]
fn err_04_08_10_malloc_failure() {
    let p = Pair::load();
    // count * sizeof(DataEntry) = count * 40 bytes; these all exceed what the
    // system allocator will hand out (>= 8 GiB on this host, 85 GiB for
    // INT_MAX), so malloc returns NULL in both implementations.
    for count in [
        i32::MAX,
        i32::MAX - 1,
        2_000_000_000,
        1_500_000_000,
        1_000_000_000,
        500_000_000,
        200_000_000,
    ] {
        // mode 1 (row 8) and mode 2 (row 12) both surface it as -1
        p.assert_same_and_eq("err04/mode1", 1, count, 0, 0, -1);
        p.assert_same_and_eq("err04/mode2", 2, count, 3, 7, -1);
    }
}

// ---------------------------------------------------------------------------
// Rows 5 & 10 — create_entries `count <= 0` / dataentry `count == 0`
// ---------------------------------------------------------------------------
// Unreachable by construction: `count = param1 > 0 ? param1 : 5` (mode 1) and
// `: 3` (mode 2) can never be <= 0, so `create_entries` never sees a
// non-positive count and `count == 0` in mode 1 is dead. The observable
// consequence is that param1 <= 0 selects the default count instead of
// returning -1; asserted here for both implementations.
#[test]
fn err_05_count_never_nonpositive() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 105);
    for p1 in [0, -1, -2, -5, -6, -1000, i32::MIN, i32::MIN + 1] {
        // mode 1 => count 5, ids 100..104
        p.assert_same_and_eq("err05/mode1-hit", 1, p1, 0, 0, 1000);
        p.assert_same_and_eq("err05/mode1-last", 1, p1, 4, 0, 1040);
        p.assert_same_and_eq("err05/mode1-past", 1, p1, 5, 0, -2);
        // mode 2 => count 3, values 2000+2010+2020 = 6030
        p.assert_same_and_eq("err05/mode2", 2, p1, 1, 0, 6030);
    }
    for _ in 0..500 {
        let p1 = rng.range(i32::MIN, 0);
        let r1 = p.assert_same("err05/rand-mode1", 1, p1, 0, rng.mixed_i32());
        assert_eq!(r1, 1000, "mode 1 with param1<=0 must use count=5, not fail");
        let r2 = p.assert_same("err05/rand-mode2", 2, p1, 1, 0);
        assert_eq!(r2, 6030, "mode 2 with param1<=0 must use count=3, not fail");
    }
}

// ---------------------------------------------------------------------------
// Row 6 — modify_entries `entries == NULL` => -1
// ---------------------------------------------------------------------------
// Unreachable by construction: mode 2 does `if (entries == NULL) { result=-1;
// break; }` before calling modify_entries, so the callee's own NULL guard is
// dead. Verified by showing the -1 for a failed allocation comes from the
// caller's guard (row 12) while every successful allocation produces the
// accumulated total in both implementations.
#[test]
fn err_06_modify_entries_null_guard() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 106);
    // allocation failure: -1 comes from dataentry's own guard
    p.assert_same_and_eq("err06/alloc-fail", 2, i32::MAX, 1, 0, -1);
    // successful allocations: never the -1 of the dead guard unless the sum
    // legitimately equals -1
    for _ in 0..1000 {
        let p1 = rng.range(1, 64);
        let p2 = rng.range(1, 1000);
        let r = p.assert_same("err06/ok", 2, p1, p2, 0);
        assert_ne!(
            r, -1,
            "positive multiplier with small counts cannot produce the NULL sentinel"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 7 — calculate_lookup returns 0 when the table cell is 0
// ---------------------------------------------------------------------------
// Unreachable by construction: no cell of `lookup_table[4][3]` is 0, so
// `calculate_lookup` always returns 1 and `*result` is always written. All 12
// cells are checked to return `cell*2 + param3`, never the 0 sentinel.
#[test]
fn err_07_lookup_never_zero() {
    let p = Pair::load();
    let table = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];
    for row in 0..4i32 {
        for col in 0..3i32 {
            let expect = table[row as usize][col as usize] * 2;
            p.assert_same_and_eq("err07", 3, row, col, 0, expect);
            assert_ne!(expect, 0);
        }
    }
    // param3 chosen to cancel the doubled cell: the result is 0 but only via
    // the successful path, and both implementations must agree.
    for row in 0..4i32 {
        for col in 0..3i32 {
            let cell2 = table[row as usize][col as usize] * 2;
            p.assert_same_and_eq("err07/cancel", 3, row, col, -cell2, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — dataentry mode 1 `found->id == 0` => -2
// ---------------------------------------------------------------------------
// Unreachable by construction: ids are `base_id + i` with `base_id == 100` and
// `0 <= i < count`, so an id of 0 would need count > 2^31-100 (which fails to
// allocate first). Every successful lookup therefore yields `found->value`.
#[test]
fn err_11_found_id_never_zero() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..1000 {
        let count = rng.range(1, 512);
        let idx = rng.range(0, count - 1);
        let r = p.assert_same("err11", 1, count, idx, rng.mixed_i32());
        assert_eq!(
            r,
            (100 + idx) * 10,
            "a found entry must never take the id==0 rejection"
        );
        assert_ne!(r, -2);
    }
    // the counts where id 0 could exist all fail allocation instead
    p.assert_same_and_eq("err11/overflow-count", 1, i32::MAX, 0, 0, -1);
}

// ---------------------------------------------------------------------------
// Rows 13-16 — mode 3 range rejections (row < 0, row >= 4, col < 0, col >= 3)
// ---------------------------------------------------------------------------
#[test]
fn err_13_16_mode3_range_rejects() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 113);

    // Row 13: param1 < 0
    for p1 in [-1, -2, -100, i32::MIN, i32::MIN + 1] {
        for p2 in [0, 1, 2] {
            p.assert_same_and_eq("err13", 3, p1, p2, rng.mixed_i32(), 0);
        }
    }
    // Row 14: param1 >= 4 (G4: exactly one past)
    for p1 in [4, 5, 6, 100, i32::MAX, i32::MAX - 1] {
        for p2 in [0, 1, 2] {
            p.assert_same_and_eq("err14", 3, p1, p2, rng.mixed_i32(), 0);
        }
    }
    // Row 15: param2 < 0
    for p2 in [-1, -2, -100, i32::MIN, i32::MIN + 1] {
        for p1 in [0, 1, 2, 3] {
            p.assert_same_and_eq("err15", 3, p1, p2, rng.mixed_i32(), 0);
        }
    }
    // Row 16: param2 >= 3 (G4: exactly one past)
    for p2 in [3, 4, 5, 100, i32::MAX, i32::MAX - 1] {
        for p1 in [0, 1, 2, 3] {
            p.assert_same_and_eq("err16", 3, p1, p2, rng.mixed_i32(), 0);
        }
    }
    // both out of range simultaneously
    for _ in 0..1000 {
        let p1 = if rng.next_u64() % 2 == 0 {
            rng.range(4, i32::MAX)
        } else {
            rng.range(i32::MIN, -1)
        };
        let p2 = if rng.next_u64() % 2 == 0 {
            rng.range(3, i32::MAX)
        } else {
            rng.range(i32::MIN, -1)
        };
        p.assert_same_and_eq("err13_16/rand", 3, p1, p2, rng.mixed_i32(), 0);
    }
    // exhaustive small neighbourhood around the valid 4x3 window
    for p1 in -3..8i32 {
        for p2 in -3..7i32 {
            let valid = (0..4).contains(&p1) && (0..3).contains(&p2);
            let expect = if valid {
                [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]][p1 as usize]
                    [p2 as usize]
                    * 2
                    + 5
            } else {
                0
            };
            p.assert_same_and_eq("err13_16/window", 3, p1, p2, 5, expect);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — mode 2 total == 0 (multiplier 0) => param3 is NOT added
// ---------------------------------------------------------------------------
#[test]
fn err_17_mode2_zero_total_skips_param3() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 117);
    for p3 in [0, 1, -1, 12345, i32::MAX, i32::MIN] {
        for p1 in [-1, 0, 1, 2, 3, 5, 64] {
            p.assert_same_and_eq("err17", 2, p1, 0, p3, 0);
        }
    }
    for _ in 0..1000 {
        let p1 = rng.range(-8, 256);
        let p3 = rng.mixed_i32();
        p.assert_same_and_eq("err17/rand", 2, p1, 0, p3, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — out-of-range "enum" values for `mode` across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_18_out_of_range_mode() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 118);
    // every one-step-past / sentinel-ish mode value
    for mode in [
        0,
        -1,
        4,
        5,
        -2,
        -3,
        6,
        7,
        8,
        255,
        256,
        -255,
        65_536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x0100_0001, // low byte == 1: must NOT be mistaken for mode 1
        0x0100_0002,
        0x0100_0003,
        -0x7FFF_FFFF,
    ] {
        for p1 in [0, 1, -1, 3, i32::MAX, i32::MIN] {
            p.assert_same_and_eq("err18", mode, p1, 11, 22, 8i32.wrapping_mul(p1));
        }
    }
    for _ in 0..5000 {
        let mut mode = rng.next_i32();
        if (1..=3).contains(&mode) {
            mode = mode.wrapping_add(1_000);
        }
        let p1 = rng.mixed_i32();
        p.assert_same_and_eq(
            "err18/rand",
            mode,
            p1,
            rng.mixed_i32(),
            rng.mixed_i32(),
            8i32.wrapping_mul(p1),
        );
    }
}

// ---------------------------------------------------------------------------
// G5 — wraparound of `100 + param2` in mode 1
// ---------------------------------------------------------------------------
#[test]
fn err_g5_param2_wraparound() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 205);
    for p2 in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 99,
        i32::MIN + 100,
        -100,
        -99,
        -101,
    ] {
        for p1 in [0, 1, 2, 5, 33] {
            let r = p.assert_same("errG5", 1, p1, p2, rng.mixed_i32());
            assert_eq!(r, -2, "wrapped target id must not match any entry");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic ABI boundaries: no pointer parameters exist on the public ABI, so
// the remaining generic surface is the full extremal cross-product of the four
// int arguments (documented in ERRORS.md).
// ---------------------------------------------------------------------------
#[test]
fn err_generic_extremal_cross_product() {
    let p = Pair::load();
    let extremes = [
        0i32,
        1,
        -1,
        2,
        3,
        4,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for &mode in extremes.iter() {
        for &p1 in extremes.iter() {
            // param1 == INT_MAX etc. in modes 1/2 exercises the allocation
            // failure path; everything else is cheap.
            for &p2 in extremes.iter() {
                for &p3 in extremes.iter() {
                    p.assert_same("err/extremes", mode, p1, p2, p3);
                }
            }
        }
    }
}
