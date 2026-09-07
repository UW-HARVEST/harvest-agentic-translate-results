//! Exhaustive brute-force differential sweeps. Slow (tens of thousands of
//! captures), so marked `#[ignore]`; run explicitly with:
//!
//! ```text
//! cargo test --test stress_exhaustive -- --ignored --nocapture
//! ```

mod harness;

use harness::*;

/// Every defined input to `bad()` in a wide window around the boundary,
/// exhaustively (not sampled).
#[test]
#[ignore = "slow: exhaustive sweep"]
fn stress_bad_exhaustive_window() {
    for v in -2000..=BAD_MAX_DEFINED {
        diff_bad(&format!("stress bad({v})"), v);
    }
    for v in [i32::MIN, i32::MIN + 1, i32::MIN + 2, -1_000_000_000] {
        diff_bad(&format!("stress bad({v})"), v);
    }
}

/// `good()` is total over `i32`; sweep a wide window exhaustively plus the
/// extremes.
#[test]
#[ignore = "slow: exhaustive sweep"]
fn stress_good_exhaustive_window() {
    for v in -2000..=2000 {
        diff_good(&format!("stress good({v})"), v);
    }
    for v in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        diff_good(&format!("stress good({v})"), v);
    }
}

/// Full cross-product of `driver` over the interesting region.
#[test]
#[ignore = "slow: exhaustive sweep"]
fn stress_driver_exhaustive_cross_product() {
    for g in -30..=30 {
        for b in -30..=BAD_MAX_DEFINED {
            diff_driver(&format!("stress driver({g}, {b})"), g, b);
        }
    }
}

/// Every single byte value and every length 0..=300 for `printLine`.
#[test]
#[ignore = "slow: exhaustive sweep"]
fn stress_print_line_exhaustive_lengths() {
    for len in 0..=300usize {
        let s: Vec<u8> = (0..len).map(|i| ((i % 255) + 1) as u8).collect();
        diff_print_line(&format!("stress printLine(len {len})"), Some(&s));
    }
}

/// Exhaustive `printIntLine` over a wide window plus decade boundaries where
/// `%d` changes width.
#[test]
#[ignore = "slow: exhaustive sweep"]
fn stress_print_int_line_exhaustive_window() {
    for v in -5000..=5000 {
        diff_print_int_line(&format!("stress printIntLine({v})"), v);
    }
    let mut edges = Vec::new();
    let mut p: i64 = 1;
    while p <= 10_000_000_000 {
        for d in [-1i64, 0, 1] {
            let x = p + d;
            if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                edges.push(x as i32);
            }
            let y = -p + d;
            if y >= i32::MIN as i64 && y <= i32::MAX as i64 {
                edges.push(y as i32);
            }
        }
        p *= 10;
    }
    for v in edges {
        diff_print_int_line(&format!("stress printIntLine(decade {v})"), v);
    }
}
