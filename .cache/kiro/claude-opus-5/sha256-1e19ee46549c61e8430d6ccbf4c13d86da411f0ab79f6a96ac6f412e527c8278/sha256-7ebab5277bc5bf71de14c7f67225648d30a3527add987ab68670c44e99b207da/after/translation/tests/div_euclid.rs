//! Differential tests for `div_euclid`, the only symbol the C library exports.
//!
//! Every call goes through `dlopen`/`dlsym` on both the C `.so` and the Rust
//! `cdylib`, so the `#[no_mangle]` export wrapper is exercised too.

mod common;

use common::{Harness, Rng, edge_values};

/// Both libraries must expose the symbol under its exact C name.
#[test]
fn exports_div_euclid_symbol() {
    // `Harness::load` fails loudly if either `.so` is missing the symbol.
    let h = Harness::load();
    h.assert_match(7, 3);
}

/// The `v2 == 0` early return, which precedes every other branch.
#[test]
fn division_by_zero() {
    let h = Harness::load();
    for v1 in edge_values() {
        h.assert_match(v1, 0);
    }
}

/// `v1 >= 0 && v2 >= 0`: the plain `v1 / v2` fast path.
#[test]
fn both_non_negative() {
    let h = Harness::load();
    let values: Vec<i32> = edge_values().into_iter().filter(|v| *v >= 0).collect();
    for &v1 in &values {
        for &v2 in &values {
            h.assert_match(v1, v2);
        }
    }
}

/// `v1 >= 0 && v2 < 0`, covering both the `v2 != INT_MIN` and `INT_MIN` arms.
#[test]
fn non_negative_over_negative() {
    let h = Harness::load();
    let all = edge_values();
    let dividends: Vec<i32> = all.iter().copied().filter(|v| *v >= 0).collect();
    let divisors: Vec<i32> = all.iter().copied().filter(|v| *v < 0).collect();
    for &v1 in &dividends {
        for &v2 in &divisors {
            h.assert_match(v1, v2);
        }
    }
}

/// `v1 < 0 && v1 != INT_MIN`: the middle branch of the outer chain.
#[test]
fn negative_dividend_not_int_min() {
    let h = Harness::load();
    let all = edge_values();
    let dividends: Vec<i32> = all
        .iter()
        .copied()
        .filter(|v| *v < 0 && *v != i32::MIN)
        .collect();
    for &v1 in &dividends {
        for &v2 in &all {
            if v2 == 0 {
                continue;
            }
            h.assert_match(v1, v2);
        }
    }
}

/// `v1 == INT_MIN`: the rebalanced branches, including `INT_MIN / -1`.
#[test]
fn int_min_dividend_edges() {
    let h = Harness::load();
    for v2 in edge_values() {
        h.assert_match(i32::MIN, v2);
    }
    // Dense sweep of small divisors, where the `+/- 1` correction bites.
    for v2 in -4096..=4096i32 {
        if v2 == 0 {
            continue;
        }
        h.assert_match(i32::MIN, v2);
    }
    for v2 in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        h.assert_match(i32::MIN, v2);
    }
}

/// `v2 == INT_MIN` against every dividend class.
#[test]
fn int_min_divisor_edges() {
    let h = Harness::load();
    for v1 in edge_values() {
        h.assert_match(v1, i32::MIN);
    }
    for v1 in -4096..=4096i32 {
        h.assert_match(v1, i32::MIN);
    }
}

/// Exhaustive over a small dense square, both signs, including zero divisor.
#[test]
fn exhaustive_small_square() {
    let h = Harness::load();
    for v1 in -220..=220i32 {
        for v2 in -220..=220i32 {
            h.assert_match(v1, v2);
        }
    }
}

/// Full cartesian product of the boundary set.
#[test]
fn edge_cartesian_product() {
    let h = Harness::load();
    let values = edge_values();
    for &v1 in &values {
        for &v2 in &values {
            h.assert_match(v1, v2);
        }
    }
}

/// Uniform random 32-bit inputs.
#[test]
fn randomized_full_range() {
    let h = Harness::load();
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);
    for _ in 0..400_000 {
        h.assert_match(rng.next_i32(), rng.next_i32());
    }
}

/// Random inputs biased towards small magnitudes and mixed magnitudes.
#[test]
fn randomized_mixed_magnitudes() {
    let h = Harness::load();
    let mut rng = Rng::new(0x0BAD_C0FF_EE00_2222);
    for _ in 0..400_000 {
        h.assert_match(rng.next_small_i32(), rng.next_small_i32());
        h.assert_match(rng.next_i32(), rng.next_small_i32());
        h.assert_match(rng.next_small_i32(), rng.next_i32());
    }
}

/// Random inputs drawn from the boundary set crossed with random values, so
/// each extreme is paired with arbitrary partners.
#[test]
fn randomized_against_edges() {
    let h = Harness::load();
    let values = edge_values();
    let mut rng = Rng::new(0xFEED_FACE_CAFE_3333);
    for &edge in &values {
        for _ in 0..2_000 {
            let other = if rng.next_u64() & 1 == 0 {
                rng.next_i32()
            } else {
                rng.next_small_i32()
            };
            h.assert_match(edge, other);
            h.assert_match(other, edge);
        }
    }
}

/// Multiples and near-multiples: exact division versus a remainder of one,
/// which is what selects the final `q` correction.
#[test]
fn near_multiples() {
    let h = Harness::load();
    let divisors: Vec<i32> = edge_values()
        .into_iter()
        .filter(|v| *v != 0 && *v != i32::MIN)
        .collect();

    for &v2 in &divisors {
        for k in -3i64..=3 {
            let base = (v2 as i64) * k;
            for delta in -2i64..=2 {
                let v1 = base + delta;
                if v1 >= i32::MIN as i64 && v1 <= i32::MAX as i64 {
                    h.assert_match(v1 as i32, v2);
                }
            }
        }
    }
}
