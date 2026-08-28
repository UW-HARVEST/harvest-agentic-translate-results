// Phase C -- error-path differential tests.
//
// One test per row of ERRORS.md. Each constructs the exact invalid input /
// condition, calls BOTH `.so`s, and asserts they return the SAME sentinel
// (not merely "both failed somehow").

mod common;
use common::*;

// ===========================================================================
// E1 / E2 -- find_entry returns NULL  =>  dataentry mode 1 returns -2
// ===========================================================================

/// E1: `param2 < 0`, so `target_id = 100 + param2 < 100` matches no id.
#[test]
fn err_e1_e2_mode1_param2_negative() {
    let mut rng = Rng::new(0xE001);
    // deterministic boundary values first
    for &p2 in &[-1, -2, -5, -99, -100, -101, -1000] {
        for count in [1, 2, 5, 10, 11, 64] {
            same_eq(1, count, p2, 0, -2);
        }
    }
    for _ in 0..2000 {
        let count = rng.in_range(1, 64);
        let p2 = rng.in_range(i32::MIN, -1);
        let p3 = rng.edgy_i32();
        same_eq(1, count, p2, p3, -2);
    }
}

/// E2: `param2 >= count` -- including exactly one step past the last valid
/// index for every small count.
#[test]
fn err_e1_e2_mode1_param2_ge_count() {
    for count in 1..=32i32 {
        // one step past the end
        same_eq(1, count, count, 0, -2);
        // and further out
        same_eq(1, count, count + 1, 0, -2);
        same_eq(1, count, count + 1000, 0, -2);
        // last valid index must still be found (proves the boundary is exact)
        same_eq(1, count, count - 1, 0, (100 + count - 1) * 10);
    }

    let mut rng = Rng::new(0xE002);
    for _ in 0..2000 {
        let count = rng.in_range(1, 64);
        let p2 = rng.in_range(count, count.saturating_add(100_000));
        same_eq(1, count, p2, rng.edgy_i32(), -2);
    }
}

/// E3: same rejection reached through the `param1 <= 0` default count of 5.
#[test]
fn err_e3_mode1_default_count_out_of_range() {
    for &p1 in &[0, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        // 0..=4 are valid for the default count of 5
        for p2 in 0..5 {
            same_eq(1, p1, p2, 0, (100 + p2) * 10);
        }
        // 5 is one step past the end
        for &p2 in &[5, 6, 7, 100, i32::MAX] {
            same_eq(1, p1, p2, 0, -2);
        }
        for &p2 in &[-1, -2, i32::MIN] {
            same_eq(1, p1, p2, 0, -2);
        }
    }
}

/// E4: `100 + param2` overflows `int` -> wrapped (negative) target id.
#[test]
fn err_e4_mode1_target_id_overflow() {
    let overflowing = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 50,
        i32::MAX - 99,
        i32::MAX - 100,
        i32::MAX - 101,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 100,
    ];
    for &p2 in &overflowing {
        for count in [1, 2, 3, 5, 10, 11, 64, 257] {
            for &p3 in &[0, i32::MAX, i32::MIN] {
                // 100 + p2 wraps; no id in [100, 100+count) can match
                same_eq(1, count, p2, p3, -2);
            }
        }
        for &p1 in &[0, -1, i32::MIN] {
            same_eq(1, p1, p2, 0, -2);
        }
    }
}

/// E5: the `found->id == 0` branch (line 153) is dead -- whenever `find_entry`
/// succeeds, `dataentry` returns `found->value`, never `-2`.
#[test]
fn err_e5_mode1_found_id_never_zero() {
    let mut rng = Rng::new(0xE005);
    for _ in 0..3000 {
        let count = rng.in_range(1, 512);
        let p2 = rng.in_range(0, count - 1);
        let p3 = rng.edgy_i32();
        let got = same_ne(1, count, p2, p3, -2);
        // id = 100 + p2 is never 0 for an in-range p2, so value is returned
        assert_eq!(got, (100 + p2) * 10, "count={count} p2={p2}");
        assert_ne!(got, -1);
    }
}

// ===========================================================================
// E6 / E7 -- malloc failure in create_entries  =>  -1
// ===========================================================================

/// E6: mode 1 with a `count` so large that `malloc(count * 40)` must fail
/// (>= 21 GiB requested; machine has far less RAM and no swap).
#[test]
fn err_e6_mode1_alloc_failure() {
    for &p1 in &ALLOC_FAIL_COUNTS {
        let bytes = (p1 as i64) * SIZEOF_DATAENTRY as i64;
        assert!(
            bytes > ALLOC_FAIL_MIN_BYTES,
            "p1={p1} must request > {ALLOC_FAIL_MIN_BYTES} bytes, got {bytes}"
        );
        for &p2 in &[0, 1, -1, i32::MAX, i32::MIN] {
            for &p3 in &[0, i32::MAX] {
                same_eq(1, p1, p2, p3, -1);
            }
        }
    }
}

/// E7: the same allocation failure reached through mode 2.
#[test]
fn err_e7_mode2_alloc_failure() {
    for &p1 in &ALLOC_FAIL_COUNTS {
        for &p2 in &[0, 1, -1, i32::MAX, i32::MIN] {
            for &p3 in &[0, 1, i32::MAX, i32::MIN] {
                same_eq(2, p1, p2, p3, -1);
            }
        }
    }
}

// ===========================================================================
// E8 / E9 -- `count <= 0` / `count == 0` guards are unreachable
// ===========================================================================

/// E8 + E9: both call sites compute `count = param1 > 0 ? param1 : <5|3>`, so
/// `count >= 1` always and neither the `count <= 0` guard in `create_entries`
/// nor the `count == 0` guard at line 146 can fire. Any `param1 <= 0` must
/// therefore behave exactly like the default count, NOT like an error.
#[test]
fn err_e8_count_le_zero_unreachable() {
    let mut rng = Rng::new(0xE008);
    let mut p1s: Vec<i32> = vec![0, -1, -2, -3, -10, -1000, i32::MIN, i32::MIN + 1];
    for _ in 0..500 {
        p1s.push(rng.in_range(i32::MIN, 0));
    }

    for &p1 in &p1s {
        // mode 1: count == 5, ids 100..=104
        for p2 in 0..5 {
            let got = same_ne(1, p1, p2, 0, -1);
            assert_eq!(got, (100 + p2) * 10, "mode 1, param1={p1}");
        }
        // mode 2: count == 3, values 2000/2010/2020
        let got = same_ne(2, p1, 1, 0, -1);
        assert_eq!(got, 2000 + 2010 + 2020, "mode 2, param1={p1}");
    }
}

// ===========================================================================
// E10 -- modify_entries' NULL guard is unreachable
// ===========================================================================

/// E10: mode 2 checks `entries == NULL` at line 168 before calling
/// `modify_entries`, so `modify_entries`' own `-1` can never be observed.
/// With a successful allocation the result is always the accumulated total
/// (+ `param3` when non-zero).
#[test]
fn err_e10_modify_entries_null_unreachable() {
    fn model(p1: i32, p2: i32, p3: i32) -> i32 {
        let count = if p1 > 0 { p1 } else { 3 };
        let mut total: i32 = 0;
        for i in 0..count {
            let v = 200i32.wrapping_add(i).wrapping_mul(10);
            if v != 0 {
                total = total.wrapping_add(v.wrapping_mul(p2));
            }
        }
        if total != 0 { total.wrapping_add(p3) } else { 0 }
    }

    let mut rng = Rng::new(0xE010);
    for _ in 0..3000 {
        let p1 = sane_param1(rng.edgy_i32());
        let p2 = rng.edgy_i32();
        let p3 = rng.edgy_i32();
        let got = same(2, p1, p2, p3);
        assert_eq!(got, model(p1, p2, p3), "p1={p1} p2={p2} p3={p3}");
    }
}

// ===========================================================================
// E11 / E12 -- falsy `modify_entries` result skips `+= param3`
// ===========================================================================

/// E11: multiplier `0` makes every `value * 0 == 0`, so `total == 0`,
/// the `if` at line 173 is falsy and `param3` is NOT added.
#[test]
fn err_e11_mode2_multiplier_zero() {
    let mut rng = Rng::new(0xE011);
    let mut p3s: Vec<i32> = vec![0, 1, -1, 12345, -12345, i32::MAX, i32::MIN];
    for _ in 0..300 {
        p3s.push(rng.any_i32());
    }
    for &p3 in &p3s {
        for count in [-1, 0, 1, 2, 3, 5, 10, 11, 64, 257, 1024] {
            same_eq(2, count, 0, p3, 0);
        }
    }
}

/// E12: a NON-zero multiplier whose accumulated `total` wraps exactly to 0 --
/// `param3` must still not be added.
#[test]
fn err_e12_mode2_total_wraps_to_zero() {
    // Find (count, multiplier) pairs where the wrapping total is exactly 0.
    fn total(count: i32, mult: i32) -> i32 {
        let mut t: i32 = 0;
        for i in 0..count {
            let v = 200i32.wrapping_add(i).wrapping_mul(10);
            if v != 0 {
                t = t.wrapping_add(v.wrapping_mul(mult));
            }
        }
        t
    }

    let mut hits = 0usize;
    // sum_{i<count} (2000 + 10i) = 10*count*(400 + count - 1)/2 ; choose
    // multipliers that drive the product to a multiple of 2^32.
    let mut rng = Rng::new(0xE012);
    let mut candidates: Vec<(i32, i32)> = Vec::new();
    for count in 1..=64i32 {
        let s = total(count, 1); // wrapping sum of the values
        if s == 0 {
            candidates.push((count, 1));
            continue;
        }
        // s * m == 0 mod 2^32  <=>  m must supply the missing factors of 2.
        let tz = (s as u32).trailing_zeros();
        if tz < 32 {
            let m = 1u32 << (32 - tz);
            let m = m as i32; // wraps to i32::MIN when 32 - tz == 31
            if m != 0 && total(count, m) == 0 {
                candidates.push((count, m));
            }
        }
    }
    // brute-force a few more via random search over multipliers
    for _ in 0..200_000 {
        if candidates.len() > 40 {
            break;
        }
        let count = rng.in_range(1, 40);
        let m = rng.any_i32();
        if m != 0 && total(count, m) == 0 {
            candidates.push((count, m));
        }
    }

    for &(count, m) in &candidates {
        assert_ne!(m, 0, "row E12 requires a NON-zero multiplier");
        assert_eq!(total(count, m), 0);
        for &p3 in &[0, 1, -1, 999, i32::MAX, i32::MIN] {
            same_eq(2, count, m, p3, 0);
            hits += 1;
        }
    }
    assert!(hits > 0, "E12 found no wrapping-to-zero case to test");
}

// ===========================================================================
// E13..E16 -- mode 3 range check (line 181)
// ===========================================================================

/// E13 (`param1 < 0`), E14 (`param1 >= 4`), E15 (`param2 < 0`),
/// E16 (`param2 >= 3`): the range check fails and `result` stays 0.
#[test]
fn err_e13_e16_mode3_out_of_range() {
    // E13: row below range
    for &r in &[-1, -2, -4, -100, i32::MIN, i32::MIN + 1] {
        for c in -1..=4 {
            same_eq(3, r, c, 0, 0);
        }
    }
    // E14: row one step past the end (and beyond)
    for &r in &[4, 5, 6, 100, i32::MAX, i32::MAX - 1] {
        for c in -1..=4 {
            same_eq(3, r, c, 0, 0);
        }
    }
    // E15: column below range (with a VALID row, so only the column rejects)
    for r in 0..4 {
        for &c in &[-1, -2, -100, i32::MIN, i32::MIN + 1] {
            same_eq(3, r, c, 0, 0);
        }
    }
    // E16: column one step past the end
    for r in 0..4 {
        for &c in &[3, 4, 5, 100, i32::MAX, i32::MAX - 1] {
            same_eq(3, r, c, 0, 0);
        }
    }
    // param3 must not leak into the rejected path
    for &p3 in &[1, -1, i32::MAX, i32::MIN, 42] {
        same_eq(3, -1, 0, p3, 0);
        same_eq(3, 4, 0, p3, 0);
        same_eq(3, 0, -1, p3, 0);
        same_eq(3, 0, 3, p3, 0);
    }
    // randomized out-of-range pairs
    let mut rng = Rng::new(0xE013);
    for _ in 0..3000 {
        let r = rng.edgy_i32();
        let c = rng.edgy_i32();
        if (0..4).contains(&r) && (0..3).contains(&c) {
            continue;
        }
        same_eq(3, r, c, rng.edgy_i32(), 0);
    }
}

// ===========================================================================
// E17 -- calculate_lookup's `return 0` is unreachable
// ===========================================================================

/// E17: no cell of `lookup_table` is 0, so `calculate_lookup` always returns 1
/// and `param3` is always added.
#[test]
fn err_e17_lookup_zero_unreachable() {
    for row in &LOOKUP_TABLE {
        for &cell in row {
            assert_ne!(cell, 0, "lookup_table must contain no zero cell");
        }
    }
    let mut rng = Rng::new(0xE017);
    for _ in 0..3000 {
        let r = rng.in_range(0, 3);
        let c = rng.in_range(0, 2);
        let p3 = rng.any_i32();
        let got = same(3, r, c, p3);
        let expect = (LOOKUP_TABLE[r as usize][c as usize] * 2).wrapping_add(p3);
        assert_eq!(got, expect, "the `return 0` branch must never be taken");
    }
}

// ===========================================================================
// E18 / E19 / E20 -- process_name guards and the strlen guard are unreachable
// ===========================================================================

/// E18 (`dest == NULL`) and E19 (`*dest == '\0'`): the only call site passes
/// `buffer` after `strcpy(buffer, "Default")`, so `dest != NULL` and
/// `*dest == 'D'`. `process_name` therefore always returns
/// `strlen("TestName") == 8` and `dataentry` returns `8 * param1`, never `-1`.
#[test]
fn err_e18_e19_process_name_guard_unreachable() {
    let mut rng = Rng::new(0xE018);
    let mut modes: Vec<i32> = vec![0, -1, 4, 5, 7, 99, -99, i32::MAX, i32::MIN];
    while modes.len() < 400 {
        let m = rng.edgy_i32();
        if m != 1 && m != 2 && m != 3 {
            modes.push(m);
        }
    }
    for &mode in &modes {
        let p1 = rng.any_i32();
        let got = same(mode, p1, rng.any_i32(), rng.any_i32());
        assert_eq!(got, 8i32.wrapping_mul(p1), "mode={mode}: expected 8*param1");
        // `8 * p1` is always a multiple of 8 mod 2^32, so it can never be the
        // -1 sentinel; observing -1 would mean the guard fired.
        assert_ne!(got, -1, "process_name's -1 must be unreachable");
    }
    // param1 == 1 makes the sentinel maximally easy to spot: 8, not -1.
    for &mode in &[0, -1, 4, i32::MAX, i32::MIN] {
        same_eq(mode, 1, 0, 0, 8);
    }
}

/// E20: `strlen(buffer)` is 8 (never 0) after `process_name`, so `result` is
/// always overwritten with `count * param1` and never keeps `process_name`'s
/// return value... except that both happen to be 8 when `param1 == 1`.
#[test]
fn err_e20_default_strlen_never_zero() {
    // param1 == 0 => 8 * 0 == 0. If the strlen guard were falsy, the result
    // would be process_name's 8 instead. This pins the branch precisely.
    for &mode in &[0, -1, -2, 4, 5, 6, 1000, -1000, i32::MAX, i32::MIN] {
        same_eq(mode, 0, 0, 0, 0);
        same_eq(mode, 0, i32::MAX, i32::MIN, 0);
    }
    // and a non-trivial multiplier confirms count == 8 exactly
    for &mode in &[0, 4, i32::MIN] {
        same_eq(mode, 3, 0, 0, 24);
        same_eq(mode, -3, 0, 0, -24);
    }
}

// ===========================================================================
// E21 -- out-of-range "enum" values for `mode` across the FFI boundary
// ===========================================================================

/// E21: a C `switch` accepts any `int`. Every value with no matching `case`
/// must take `default:` identically in both implementations.
#[test]
fn err_e21_mode_out_of_range_enum() {
    // exhaustive-ish sweep around the valid arms
    for mode in -64..=64i32 {
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        for &p1 in &[0, 1, -1, 7, i32::MAX, i32::MIN] {
            same_eq(mode, p1, 0, 0, 8i32.wrapping_mul(p1));
        }
    }
    // extreme / bit-pattern enum values
    let exotic = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -1,
        0,
        4,
        0x7FFF_FFFE,
        0x0000_0100,
        0x0001_0001,
        0x5555_5555,
        -0x5555_5556,
        1 << 16,
        1 << 24,
        1 << 30,
    ];
    let mut rng = Rng::new(0xE021);
    for &mode in &exotic {
        assert!(mode != 1 && mode != 2 && mode != 3);
        for _ in 0..100 {
            let p1 = rng.edgy_i32();
            same_eq(mode, p1, rng.edgy_i32(), rng.edgy_i32(), 8i32.wrapping_mul(p1));
        }
    }
    // fully random modes
    for _ in 0..20_000 {
        let mode = rng.any_i32();
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        let p1 = rng.any_i32();
        same_eq(mode, p1, rng.any_i32(), rng.any_i32(), 8i32.wrapping_mul(p1));
    }
}

// ===========================================================================
// E22 -- `8 * param1` overflow in the default arm
// ===========================================================================

#[test]
fn err_e22_default_overflow() {
    let mut p1s: Vec<i32> = vec![
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1 << 28,
        (1 << 28) + 1,
        3 << 28,
        1 << 29,
        1 << 30,
        -(1 << 28),
        -(1 << 29),
        -(1 << 30),
        0x1000_0000,
        0x2000_0000,
        0x4000_0000,
        0x7FFF_FFFF,
    ];
    let mut rng = Rng::new(0xE022);
    for _ in 0..2000 {
        p1s.push(rng.any_i32());
    }
    for &p1 in &p1s {
        for &mode in &[0, -1, 4, i32::MAX, i32::MIN] {
            same_eq(mode, p1, 0, 0, 8i32.wrapping_mul(p1));
        }
    }
}

// ===========================================================================
// E23 -- zero / "zero length" parameters in every mode
// ===========================================================================

#[test]
fn err_e23_zero_params_all_modes() {
    // mode 1: param1 == 0 -> count 5; param2 == 0 -> first id found
    same_eq(1, 0, 0, 0, 1000);
    // mode 2: param1 == 0 -> count 3, multiplier 0 -> total 0, param3 dropped
    same_eq(2, 0, 0, 0, 0);
    // mode 3: (0,0) is a valid cell -> 2 * 10 + 0
    same_eq(3, 0, 0, 0, 20);
    // default: 8 * 0
    same_eq(0, 0, 0, 0, 0);
    // all-zero arguments, and every single-argument-zero variant
    for mode in [0, 1, 2, 3] {
        for p1 in [0, 1] {
            for p2 in [0, 1] {
                for p3 in [0, 1] {
                    same(mode, p1, p2, p3);
                }
            }
        }
    }
}

// ===========================================================================
// E24 -- INT_MIN / INT_MAX matrix over every argument position
// ===========================================================================

#[test]
fn err_e24_extreme_values_matrix() {
    const EXT: [i32; 6] = [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX];

    // mode is varied over the valid arms plus the extremes themselves.
    let modes = [1i32, 2, 3, 0, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];

    for &mode in &modes {
        for &p1 in &EXT {
            // For modes 1/2 an INT_MAX-ish param1 requests ~86 GiB and must
            // fail allocation in BOTH libraries -- that is itself the assertion.
            for &p2 in &EXT {
                for &p3 in &EXT {
                    same(mode, p1, p2, p3);
                }
            }
        }
    }

    // Explicit sentinel expectations for the extreme allocation requests.
    same_eq(1, i32::MAX, 0, 0, -1);
    same_eq(2, i32::MAX, 1, 0, -1);
    // INT_MIN param1 is <= 0, so it takes the DEFAULT count, not an error.
    same_eq(1, i32::MIN, 0, 0, 1000);
    same_eq(2, i32::MIN, 1, 0, 2000 + 2010 + 2020);
}
