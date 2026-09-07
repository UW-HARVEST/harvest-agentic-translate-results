//! Heavy brute-force sweeps. These are not tied to a single `CONFIGS.md` row;
//! they exist to catch value-dependent divergences that per-row sampling could
//! miss. Every call still goes through the `dataentry` dynamic symbol of both
//! shared objects.

mod harness;

use harness::{Rng, c_fn, rs_fn};
use std::ffi::c_int;

/// Fast path: compare without the per-call formatting cost of `assert_same`.
fn diff(mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> Option<(c_int, c_int)> {
    let c = c_fn();
    let r = rs_fn();
    let cv = unsafe { c(mode, p1, p2, p3) };
    let rv = unsafe { r(mode, p1, p2, p3) };
    if cv == rv { None } else { Some((cv, rv)) }
}

fn expect_same(mode: c_int, p1: c_int, p2: c_int, p3: c_int) {
    if let Some((cv, rv)) = diff(mode, p1, p2, p3) {
        panic!("divergence at dataentry({mode}, {p1}, {p2}, {p3}): C={cv} Rust={rv}");
    }
}

/// Mode 1: every count in 1..=64 crossed with every index in -70..=70.
#[test]
fn sweep_mode1_dense() {
    for p1 in 1..=64 {
        for p2 in -70..=70 {
            for p3 in [0, 7, c_int::MAX] {
                expect_same(1, p1, p2, p3);
            }
        }
    }
    // non-positive param1 (count defaults to 5) crossed with the same indices
    for p1 in -64..=0 {
        for p2 in -70..=70 {
            expect_same(1, p1, p2, 0);
        }
    }
}

/// Mode 2: every count in 1..=48 crossed with a dense band of multipliers and
/// several addends, including overflow-inducing ones.
#[test]
fn sweep_mode2_dense() {
    for p1 in 1..=48 {
        for p2 in -70..=70 {
            for p3 in [0, 7, -7, c_int::MAX, c_int::MIN] {
                expect_same(2, p1, p2, p3);
            }
        }
    }
    for p1 in -48..=0 {
        for p2 in -70..=70 {
            for p3 in [0, 13, c_int::MIN] {
                expect_same(2, p1, p2, p3);
            }
        }
    }
}

/// Mode 2 with large multipliers that wrap the accumulator repeatedly.
#[test]
fn sweep_mode2_wrapping_totals() {
    let mut rng = Rng::new(0xE0002);
    for _ in 0..100_000 {
        let p1 = rng.range(1, 64);
        let p2 = rng.i32();
        let p3 = rng.i32();
        expect_same(2, p1, p2, p3);
    }
}

/// Mode 3: exhaustive over a cube well past both range bounds.
#[test]
fn sweep_mode3_exhaustive_cube() {
    for p1 in -16..=16 {
        for p2 in -16..=16 {
            for p3 in -16..=16 {
                expect_same(3, p1, p2, p3);
            }
        }
    }
    let mut rng = Rng::new(0xE0003);
    for _ in 0..200_000 {
        expect_same(3, rng.range(-6, 9), rng.range(-6, 9), rng.i32());
    }
}

/// Default arm: dense `mode` values crossed with a dense multiplier band.
#[test]
fn sweep_default_dense() {
    for mode in -64..=64 {
        if mode == 1 || mode == 2 || mode == 3 {
            continue;
        }
        for p1 in -70..=70 {
            expect_same(mode, p1, 0, 0);
        }
    }
}

/// Fully random 4-tuples over the whole `i32` space, with `param1` clamped only
/// for the two allocating modes so the sweep stays runnable.
#[test]
fn sweep_random_full_range() {
    let mut rng = Rng::new(0xE0009);
    for _ in 0..400_000 {
        let mode = if rng.range(0, 2) == 0 {
            rng.range(-6, 9)
        } else {
            rng.i32()
        };
        let p1 = if mode == 1 || mode == 2 {
            rng.range(-32, 128)
        } else {
            rng.i32()
        };
        expect_same(mode, p1, rng.i32(), rng.i32());
    }
}

/// Allocation-threshold band: counts from 2^20 up to and past the point where
/// `count * sizeof(DataEntry)` can no longer be allocated. This is the one
/// region a bounded sweep cannot reach, and it is where a C-`malloc` vs
/// Rust-`Vec` size-computation mismatch would show up. Both sides must agree on
/// which side of the threshold each count falls, and on the wrapped totals for
/// the counts that do allocate.
#[test]
fn sweep_allocation_threshold_band() {
    for shift in 20..=26 {
        let p1: c_int = 1 << shift;
        expect_same(1, p1, 0, 0);
        expect_same(1, p1, p1 - 1, 0);
        expect_same(2, p1, 3, 7);
        expect_same(2, p1, -3, 7);
    }
    // Past the threshold: allocation must fail identically.
    for p1 in [
        1 << 28,
        1 << 29,
        1 << 30,
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX / 2,
    ] {
        expect_same(1, p1, 0, 0);
        expect_same(2, p1, 3, 7);
    }
}

/// Random 4-tuples for the allocating modes where `param1` is either
/// non-positive (count falls back to the literal default) or large enough that
/// `count * sizeof(DataEntry)` cannot be allocated. The genuinely allocatable
/// middle band is covered by `sweep_mode1_dense` / `sweep_mode2_dense` and
/// `row07`/`row14`; drawing it here would just allocate tens of gigabytes.
#[test]
fn sweep_random_unclamped() {
    let mut rng = Rng::new(0xE000A);
    // Verified unallocatable on this platform (>= 1 GiB entries => >= 40 GiB).
    const HUGE: [c_int; 6] = [
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX / 2,
        0x4000_0000,
        0x5000_0000,
        0x6000_0000,
    ];
    for _ in 0..3_000 {
        let mode = rng.range(1, 3);
        let p1 = if rng.range(0, 1) == 0 {
            rng.range(c_int::MIN as i64, 0)
        } else {
            HUGE[(rng.range(0, (HUGE.len() - 1) as i64)) as usize]
        };
        expect_same(mode, p1, rng.i32(), rng.i32());
    }
}
