//! Level 2: the public entry point `overunder`, which drives every leaf
//! function and produces a large amount of formatted stdout.
//!
//! Both the return value and the exact stdout byte stream are compared.

mod common;

use common::*;
use std::ffi::c_int;

fn run_pair(a: c_int, b: c_int, c: c_int, d: c_int) {
    let im = impls();
    let cf = im.c_sym::<FnOverunder>("overunder");
    let rf = im.rust_sym::<FnOverunder>("overunder");

    let (c_ret, c_out) = capture_stdout(|| unsafe { cf(a, b, c, d) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { rf(a, b, c, d) });

    assert_eq!(
        c_out,
        r_out,
        "stdout differs for overunder({a}, {b}, {c}, {d})\n--- C ---\n{}\n--- Rust ---\n{}",
        show(&c_out),
        show(&r_out)
    );
    assert_eq!(
        c_ret, r_ret,
        "return value differs for overunder({a}, {b}, {c}, {d}): C={c_ret} Rust={r_ret}"
    );
}

/// Small exhaustive-ish grid: covers every `a % 6` arm (including the negative
/// remainders that hit the `default:` case) crossed with small b/c/d.
fn overunder_small_grid() {
    let vals: [c_int; 15] = [-7, -6, -5, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 12];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    run_pair(a, b, c, d);
                }
            }
        }
    }
}

/// Extremes: exercises `int` overflow in `d*d + a*a` (which makes `sqrt`
/// produce NaN), the saturating branches of `safe_double_to_int`, and wrapping
/// accumulation into `total`.
fn overunder_extremes() {
    let vals: [c_int; 18] = [
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 5,
        c_int::MIN / 2,
        -1_000_000_007,
        -46_341,
        -46_340,
        -65_536,
        -1,
        0,
        1,
        46_340,
        46_341,
        65_536,
        1_000_000_007,
        c_int::MAX / 2,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    run_pair(a, b, c, d);
                }
            }
        }
    }
}

/// Deterministic pseudo-random sweep over the full `int` range.
fn overunder_random_sweep() {
    let mut state: u64 = 0x853C_49E6_748F_EA9B;
    let mut next = || -> c_int {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32 as c_int
    };
    for _ in 0..3000 {
        let (a, b, c, d) = (next(), next(), next(), next());
        run_pair(a, b, c, d);
    }
}

/// Mixed magnitudes: one large argument at a time, so each `safe_double_to_int`
/// call site is driven independently into its saturating branch.
fn overunder_mixed_magnitudes() {
    let big: [c_int; 8] = [
        c_int::MIN,
        c_int::MIN + 1,
        -2_000_000_000,
        -1_431_655_766,
        1_431_655_765,
        2_000_000_000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let small: [c_int; 7] = [-6, -2, -1, 0, 1, 3, 5];

    for &x in &big {
        for &s in &small {
            run_pair(x, s, s, s);
            run_pair(s, x, s, s);
            run_pair(s, s, x, s);
            run_pair(s, s, s, x);
            run_pair(x, x, s, s);
            run_pair(s, s, x, x);
            run_pair(x, s, x, s);
            run_pair(s, x, s, x);
        }
    }
}

/// Single entry point: see `capture_stdout` for why each test binary must
/// contain exactly one `#[test]`.
#[test]
fn overunder_matches_c() {
    overunder_small_grid();
    overunder_extremes();
    overunder_mixed_magnitudes();
    overunder_random_sweep();
}
