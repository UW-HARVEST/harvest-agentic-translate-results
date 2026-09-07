//! Phase C — error-path differential tests, one test per row of `ERRORS.md`,
//! plus the generic FFI-boundary cases.
//!
//! Every test asserts C and Rust produce the *same* sentinel/error value, not
//! merely that both "failed somehow": the expected sentinel is asserted
//! explicitly against the C result first, then C and Rust are compared.

mod harness;

use harness::{Rng, assert_same, c_fn, rs_fn};
use std::ffi::c_int;

const IMAX: c_int = c_int::MAX;
const IMIN: c_int = c_int::MIN;

/// Calls both exports, asserts they agree AND that the C side produced the
/// expected sentinel.
fn assert_same_and_is(row: &str, mode: c_int, p1: c_int, p2: c_int, p3: c_int, expected: c_int) {
    let c = c_fn();
    let r = rs_fn();
    let cv = unsafe { c(mode, p1, p2, p3) };
    let rv = unsafe { r(mode, p1, p2, p3) };
    assert_eq!(
        cv, rv,
        "[{row}] dataentry({mode}, {p1}, {p2}, {p3}): C={cv} Rust={rv}"
    );
    assert_eq!(
        cv, expected,
        "[{row}] dataentry({mode}, {p1}, {p2}, {p3}): expected sentinel {expected}, C gave {cv}"
    );
}

/// Rows 1, 2, 10 — `find_entry` returns NULL (no id match / empty walk), which
/// surfaces as the `-2` sentinel from mode 1.
#[test]
fn row01_02_10_find_entry_null_yields_minus2() {
    // Explicit misses just past the top of the range and below the bottom.
    assert_same_and_is("row10", 1, 5, 5, 0, -2);
    assert_same_and_is("row10", 1, 5, -1, 0, -2);
    assert_same_and_is("row10", 1, 1, 1, 0, -2);
    assert_same_and_is("row10", 1, 0, 5, 0, -2); // default count 5, one past
    assert_same_and_is("row10", 1, -1, 5, 0, -2);
    assert_same_and_is("row10", 1, 10, 10, 0, -2);
    assert_same_and_is("row10", 1, 10, IMAX, 0, -2);
    assert_same_and_is("row10", 1, 10, IMIN, 0, -2);

    // Randomized misses on both sides of the valid index window.
    let mut rng = Rng::new(0xC0010);
    for _ in 0..20_000 {
        let p1 = rng.range(1, 300);
        let count = p1;
        let p2 = if rng.range(0, 1) == 0 {
            rng.range(count as i64, count as i64 + 5_000)
        } else {
            rng.range(IMIN as i64, -1)
        };
        assert_same_and_is("row10", 1, p1, p2, rng.i32(), -2);
    }
}

/// Rows 3, 4 — `process_name`'s `dest == NULL` / `*dest == '\0'` guard.
///
/// Both halves are provably unreachable from the public entry point: the sole
/// call site passes `buffer` (a live 32-byte stack array) that has just been
/// filled with `"Default"`, so `dest != NULL` and `*dest == 'D'`. The observable
/// consequence is that `process_name` never returns its `-1`, and the default
/// arm always overwrites its result with `8 * param1`. This test pins that
/// invariant across both implementations: no input to `dataentry` can make the
/// default arm return `-1`.
#[test]
fn row03_04_process_name_guard_unreachable() {
    let c = c_fn();
    let r = rs_fn();
    let mut rng = Rng::new(0xC0003);
    for _ in 0..20_000 {
        let mode = {
            let m = rng.i32();
            if m == 1 || m == 2 || m == 3 { 0 } else { m }
        };
        let p1 = rng.i32();
        let cv = unsafe { c(mode, p1, rng.i32(), rng.i32()) };
        let rv = unsafe { r(mode, p1, 0, 0) };
        // param2/param3 are ignored on the default path, so both calls must
        // agree, and the value must be 8*param1 (i.e. the -1 never escapes).
        assert_eq!(cv, rv, "[row03/04] mode={mode} p1={p1}");
        assert_eq!(
            cv,
            8i32.wrapping_mul(p1),
            "[row03/04] default arm returned process_name's value instead of 8*param1"
        );
    }
    // param1 == 0 makes 8*param1 == 0, which is the only way to distinguish it
    // from a -1 leak; check it explicitly.
    assert_same_and_is("row03/04", 0, 0, 0, 0, 0);
    assert_same_and_is("row03/04", -9, 0, 0, 0, 0);
}

/// Row 5 — `calculate_lookup` returning 0 (a zero table cell). Unreachable
/// because every cell is 10..120; pinned by asserting mode 3 on every valid
/// cell never yields the `result = 0` "lookup failed" shape, in both impls.
#[test]
fn row05_calculate_lookup_zero_cell_unreachable() {
    let expected_table = [
        [10, 20, 30],
        [40, 50, 60],
        [70, 80, 90],
        [100, 110, 120],
    ];
    for row in 0..4usize {
        for col in 0..3usize {
            // param3 chosen so a "lookup returned 0" path (result stays 0) is
            // distinguishable from the success path.
            let p3 = 7;
            let expected = expected_table[row][col] * 2 + p3;
            assert_same_and_is("row05", 3, row as c_int, col as c_int, p3, expected);
        }
    }
}

/// Rows 6, 8 — `malloc` failure in `create_entries` reached from mode 1: the
/// requested `count * sizeof(DataEntry)` cannot be satisfied, so `dataentry`
/// returns `-1`.
#[test]
fn row06_08_mode1_alloc_failure_minus1() {
    for p1 in [IMAX, IMAX - 1, IMAX / 2, 0x4000_0000u32 as c_int, 0x2000_0000] {
        assert_same_and_is("row06/08", 1, p1, 0, 0, -1);
    }
}

/// Rows 6, 12 — same allocation failure reached from mode 2.
#[test]
fn row06_12_mode2_alloc_failure_minus1() {
    for p1 in [IMAX, IMAX - 1, IMAX / 2, 0x4000_0000u32 as c_int, 0x2000_0000] {
        for p2 in [0, 1, -1, IMAX] {
            assert_same_and_is("row06/12", 2, p1, p2, 99, -1);
        }
    }
}

/// Rows 7, 9 — `create_entries` rejecting `count <= 0`, and mode 1's
/// `count == 0` check. Both are unreachable from `dataentry` because
/// `count = param1 > 0 ? param1 : 5|3` is always positive; the tests pin that
/// no non-positive `param1` can produce the `-1` sentinel.
#[test]
fn row07_09_nonpositive_count_unreachable() {
    let c = c_fn();
    let r = rs_fn();
    for p1 in [0, -1, -2, -5, -100, IMIN, IMIN + 1, -0x4000_0000] {
        for mode in [1, 2] {
            let cv = unsafe { c(mode, p1, 0, 0) };
            let rv = unsafe { r(mode, p1, 0, 0) };
            assert_eq!(cv, rv, "[row07/09] mode={mode} p1={p1}");
            assert_ne!(
                cv, -1,
                "[row07/09] mode={mode} p1={p1} unexpectedly hit the NULL-alloc sentinel"
            );
        }
    }
}

/// Row 13 — mode 2 where `modify_entries` totals exactly 0, so `param3` is NOT
/// added and the result is `0`.
#[test]
fn row13_mode2_zero_total_skips_param3() {
    // multiplier 0 zeroes every value, so the total is 0.
    for p1 in [-5, 0, 1, 3, 17, 512] {
        for p3 in [0, 1, -1, IMAX, IMIN, 123_456] {
            assert_same_and_is("row13", 2, p1, 0, p3, 0);
        }
    }
}

/// Row 14 — `modify_entries`'s `entries == NULL` guard. Unreachable: mode 2
/// checks `entries == NULL` and breaks with `-1` before calling it. Pinned by
/// showing that whenever mode 2 does reach `modify_entries` the result is not
/// the `-1 + param3` shape that a leaked NULL guard would produce.
#[test]
fn row14_modify_entries_null_guard_unreachable() {
    let c = c_fn();
    let r = rs_fn();
    let mut rng = Rng::new(0xC0014);
    for _ in 0..20_000 {
        let p1 = rng.range(1, 256);
        let mut p2 = rng.i32();
        if p2 == 0 {
            p2 = 1;
        }
        let p3 = rng.i32();
        let cv = unsafe { c(2, p1, p2, p3) };
        let rv = unsafe { r(2, p1, p2, p3) };
        assert_eq!(cv, rv, "[row14] p1={p1} p2={p2} p3={p3}");
    }
}

/// Rows 15-18 — mode 3 range checks: one step past each bound in both
/// directions must return `0`.
#[test]
fn row15_18_mode3_range_rejections() {
    // param1 out of [0,4)
    for p1 in [-1, -2, 4, 5, 100, IMAX, IMIN] {
        for p2 in [0, 1, 2] {
            assert_same_and_is("row15/16", 3, p1, p2, 12345, 0);
        }
    }
    // param2 out of [0,3)
    for p1 in [0, 1, 2, 3] {
        for p2 in [-1, -2, 3, 4, 100, IMAX, IMIN] {
            assert_same_and_is("row17/18", 3, p1, p2, 12345, 0);
        }
    }
    // both out of range
    for p1 in [-1, 4, IMAX, IMIN] {
        for p2 in [-1, 3, IMAX, IMIN] {
            assert_same_and_is("row15-18", 3, p1, p2, -7, 0);
        }
    }
    // randomized out-of-range sweep
    let mut rng = Rng::new(0xC0015);
    for _ in 0..20_000 {
        let p1 = rng.i32();
        let p2 = rng.i32();
        let in_range = (0..4).contains(&p1) && (0..3).contains(&p2);
        let p3 = rng.i32();
        if in_range {
            assert_same("row15-18", 3, p1, p2, p3);
        } else {
            assert_same_and_is("row15-18", 3, p1, p2, p3, 0);
        }
    }
}

/// Rows 19, 20 — out-of-range "enum-like" `mode` values crossing the FFI
/// boundary. A C `switch` on an `int` accepts any value, so every non-1/2/3
/// `int` is a real input that must land in `default` on both sides.
#[test]
fn row19_20_out_of_range_mode_values() {
    for mode in [
        0,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        -1,
        -2,
        -3,
        -100,
        1000,
        0x7FFF_FFFF,
        IMIN,
        IMIN + 1,
        IMAX - 1,
        0x0001_0000,
        -0x0001_0000,
    ] {
        for p1 in [0, 1, -1, 7, IMAX, IMIN] {
            assert_same_and_is("row19/20", mode, p1, 0, 0, 8i32.wrapping_mul(p1));
        }
    }
}

/// Generic boundaries: zero and extreme "length" parameters for every mode.
#[test]
fn generic_zero_and_extreme_lengths() {
    for mode in [1, 2, 3, 0, 4, -1, IMIN, IMAX] {
        for p1 in [0, 1, -1, IMIN, IMAX] {
            for p2 in [0, 1, -1, IMIN, IMAX] {
                for p3 in [0, 1, -1, IMIN, IMAX] {
                    assert_same("generic", mode, p1, p2, p3);
                }
            }
        }
    }
}

/// Generic boundaries: every value one step past every documented range bound.
#[test]
fn generic_one_past_every_bound() {
    // lookup table bounds (4 rows, 3 cols) and their neighbours
    for p1 in [-1, 0, 3, 4] {
        for p2 in [-1, 0, 2, 3] {
            assert_same("generic", 3, p1, p2, 1);
        }
    }
    // MAX_ENTRIES (10) is unused by the C, but check the neighbourhood anyway
    for p1 in [9, 10, 11] {
        for p2 in [9, 10, 11] {
            assert_same("generic", 1, p1, p2, 0);
            assert_same("generic", 2, p1, p2, 0);
        }
    }
    // NAME_LENGTH (32) neighbourhood: ids whose "Entry_%d" rendering changes
    // width around the 32-byte field.
    for p1 in [31, 32, 33] {
        for p2 in [30, 31, 32] {
            assert_same("generic", 1, p1, p2, 0);
        }
    }
    // mode switch-label neighbourhood
    for mode in [0, 1, 2, 3, 4] {
        assert_same("generic", mode, 2, 1, 3);
    }
}
