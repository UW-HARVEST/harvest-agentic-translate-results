//! Phase B addendum — EXHAUSTIVE sweep over a small but complete input box.
//!
//! Randomized property tests can miss a single unlucky combination; this test
//! enumerates every point of a bounded lattice around all the interesting
//! boundaries (the four `switch` arms, the `param1 > 0` ternary, the 4x3 table
//! bounds, and the id window of mode 1) and compares C vs. Rust on each.

mod common;
use common::Pair;

#[test]
fn exhaustive_small_box() {
    let p = Pair::load();
    let mut n: u64 = 0;
    for mode in -3..=6i32 {
        for param1 in -3..=14i32 {
            for param2 in -3..=14i32 {
                for param3 in -2..=2i32 {
                    p.assert_same(mode, param1, param2, param3);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 10 * 18 * 18 * 5);
    println!("exhaustive_small_box: {n} input points agreed");
}

/// Same box, but with `param3` pinned to the extremes instead of small values,
/// so every wrap-around interaction with `param1`/`param2` is enumerated too.
#[test]
fn exhaustive_small_box_extreme_param3() {
    let p = Pair::load();
    let mut n: u64 = 0;
    for mode in -2..=5i32 {
        for param1 in -2..=12i32 {
            for param2 in -2..=12i32 {
                for &param3 in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
                    p.assert_same(mode, param1, param2, param3);
                    n += 1;
                }
            }
        }
    }
    println!("exhaustive_small_box_extreme_param3: {n} input points agreed");
}

/// Exhaustive over `mode` x every table coordinate one step outside the bounds,
/// crossed with a dense `param3` band around the wrap point.
#[test]
fn exhaustive_mode3_bounds_x_param3_wrap() {
    let p = Pair::load();
    for param1 in -2..=6i32 {
        for param2 in -2..=5i32 {
            // dense band straddling i32::MAX so `lookup*2 + param3` wraps
            for delta in -300..=300i32 {
                let param3 = i32::MAX.wrapping_add(delta);
                p.assert_same(3, param1, param2, param3);
            }
            for delta in -300..=300i32 {
                let param3 = i32::MIN.wrapping_add(delta);
                p.assert_same(3, param1, param2, param3);
            }
        }
    }
}

/// Exhaustive over mode 1's whole id window for every small count, so the
/// hit/miss frontier of `find_entry` is enumerated with no gaps.
#[test]
fn exhaustive_mode1_id_window() {
    let p = Pair::load();
    for count in 1..=24i32 {
        for param2 in -(count + 3)..=(count + 3) {
            let v = p.assert_same(1, count, param2, 0);
            let expected = if (0..count).contains(&param2) {
                (100 + param2) * 10
            } else {
                -2
            };
            assert_eq!(v, expected, "count={count} param2={param2}");
        }
    }
}

/// Exhaustive over mode 2's small counts x a dense multiplier band, including
/// the values that make the running total wrap.
#[test]
fn exhaustive_mode2_count_x_multiplier() {
    let p = Pair::load();
    for count in 1..=16i32 {
        for mult in -600..=600i32 {
            p.assert_same(2, count, mult, 0);
            p.assert_same(2, count, mult, 1);
            p.assert_same(2, count, mult, -1);
        }
        for &mult in &[
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            1 << 30,
            1 << 28,
            1 << 24,
            -(1 << 30),
        ] {
            for &d in &[0, 1, -1, i32::MAX, i32::MIN] {
                p.assert_same(2, count, mult, d);
            }
        }
    }
}

/// Exhaustive over the low byte of `mode`, to be sure nothing behaves like a
/// narrow (char/short) switch: every value in `[-256, 256]` must route the same
/// way in both libraries.
#[test]
fn exhaustive_mode_low_range() {
    let p = Pair::load();
    for mode in -256..=256i32 {
        for &param1 in &[0, 1, 2, 3, 5, 7, -1, -7] {
            for &param2 in &[0, 1, 2, 3, -1] {
                p.assert_same(mode, param1, param2, 11);
            }
        }
    }
    // and around the 32-bit extremes / sign-bit patterns
    for base in [i32::MIN, i32::MAX - 256, 0x7FFF_FF00u32 as i32, -0x8000_0000i64 as i32] {
        for delta in 0..=256i32 {
            let mode = base.wrapping_add(delta);
            if (1..=3).contains(&mode) {
                continue;
            }
            p.assert_same(mode, 3, 1, 5);
        }
    }
}
