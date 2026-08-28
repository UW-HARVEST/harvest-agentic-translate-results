//! Heavy sweeps. Run with:
//! `cargo test --release --test div_euclid_heavy -- --ignored --nocapture`
//!
//! These are `#[ignore]`d so the default `cargo test` stays fast; the coverage
//! they add is breadth over the full `i32` range rather than new branches.
//! Each test is sized to finish well inside a 600 s budget on its own, which is
//! why the exhaustive sweeps are split across several functions.

mod common;

use common::{Harness, Rng, edge_values};

/// Exhaustive over every `i32` dividend for one fixed divisor.
fn sweep_all_dividends(divisors: &[i32]) {
    let h = Harness::load();
    for &v2 in divisors {
        let mut v1 = i32::MIN;
        loop {
            h.assert_match(v1, v2);
            if v1 == i32::MAX {
                break;
            }
            v1 += 1;
        }
        eprintln!("exhaustive dividend sweep done for v2 = {v2}");
    }
}

/// Exhaustive over every `i32` divisor for one fixed dividend.
fn sweep_all_divisors(dividends: &[i32]) {
    let h = Harness::load();
    for &v1 in dividends {
        let mut v2 = i32::MIN;
        loop {
            h.assert_match(v1, v2);
            if v2 == i32::MAX {
                break;
            }
            v2 += 1;
        }
        eprintln!("exhaustive divisor sweep done for v1 = {v1}");
    }
}

/// Every boundary divisor crossed with a fine, prime-strided sweep of the full
/// `i32` dividend range.
#[test]
#[ignore]
fn strided_full_range_sweep() {
    let h = Harness::load();
    const STRIDE: i64 = 1_021;

    for v2 in edge_values() {
        let mut v1 = i32::MIN as i64;
        while v1 <= i32::MAX as i64 {
            h.assert_match(v1 as i32, v2);
            v1 += STRIDE;
        }
        // Always finish on the exact upper bound.
        h.assert_match(i32::MAX, v2);
    }
}

// `+/- 1` divisors: the arms where the rebalancing arithmetic overflows in C.
#[test]
#[ignore]
fn exhaustive_dividend_divisor_unit() {
    sweep_all_dividends(&[1, -1]);
}

#[test]
#[ignore]
fn exhaustive_dividend_divisor_small() {
    sweep_all_dividends(&[2, -2, 3]);
}

#[test]
#[ignore]
fn exhaustive_dividend_divisor_small_negative() {
    sweep_all_dividends(&[-3, 7, -7]);
}

#[test]
#[ignore]
fn exhaustive_dividend_divisor_extremes() {
    sweep_all_dividends(&[i32::MIN, i32::MIN + 1, i32::MAX]);
}

// Dividends that select the `INT_MIN` arms of the outer branch chain.
#[test]
#[ignore]
fn exhaustive_divisor_dividend_int_min() {
    sweep_all_divisors(&[i32::MIN, i32::MIN + 1]);
}

#[test]
#[ignore]
fn exhaustive_divisor_dividend_small() {
    sweep_all_divisors(&[-1, 0, 1]);
}

#[test]
#[ignore]
fn exhaustive_divisor_dividend_extremes() {
    sweep_all_divisors(&[i32::MAX, i32::MAX - 1, 12_345_678]);
}

/// A large uniform random sample of the full input space.
#[test]
#[ignore]
fn large_random_sample() {
    let h = Harness::load();
    let mut rng = Rng::new(0xA5A5_5A5A_1357_9BDF);
    for _ in 0..200_000_000u64 {
        h.assert_match(rng.next_i32(), rng.next_i32());
    }
}
