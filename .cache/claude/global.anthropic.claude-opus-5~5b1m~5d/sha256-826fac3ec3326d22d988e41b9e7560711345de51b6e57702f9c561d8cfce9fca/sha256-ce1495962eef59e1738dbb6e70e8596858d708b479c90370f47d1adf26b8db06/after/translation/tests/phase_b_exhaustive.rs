//! Phase B addendum — *exhaustive* (not sampled) sweeps of the bounded axes.
//!
//! `CONFIGS.md` rows are covered with randomized inputs in
//! `phase_b_configs.rs`. These tests additionally enumerate whole axes
//! end-to-end, which is what catches value-dependent bugs (the `% 1000` fold,
//! the strict-`<` append test, the `count` saturation edge) rather than just
//! sampling near them.

mod common;

use common::*;
use std::ffi::c_int;

/// Every distinct class of `mode` the `switch` in `lib.c:126-140` recognises.
const MODES: [c_int; 6] = [0, 1, 2, 3, -1, c_int::MAX];

/// Thresholds straddling every value the produced sequences can take:
/// `process_value` tops out at 65 545 / 1 005, `double_value` at 131 070 /
/// 1 998, `triple_value` at 196 605 / 2 997.
const THRESHOLDS: [c_int; 12] = [
    c_int::MIN,
    0,
    1,
    1_000,
    1_006,
    1_999,
    2_998,
    65_545,
    65_546,
    131_071,
    196_606,
    c_int::MAX,
];

// ===========================================================================
// Exhaustive over the whole valid `seed` domain (all 65 536 values)
// ===========================================================================

#[test]
fn ex_01_every_valid_seed_one_iteration() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in [0, 1, 2, 4242] {
            for &thr in &THRESHOLDS {
                for seed in 0..=65_535 {
                    h.assert_gotomach_sweep(args(1, seed, mode, thr));
                }
            }
        }
    });
}

#[test]
fn ex_02_every_valid_seed_three_iterations() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in [0, 1, 2, -1] {
            for &thr in &[c_int::MIN, 0, 1_000, 1_006, 2_998, 196_606, c_int::MAX] {
                for seed in 0..=65_535 {
                    h.assert_gotomach_sweep(args(3, seed, mode, thr));
                }
            }
        }
    });
}

// ===========================================================================
// Exhaustive over the whole `threshold` band where produced values live
// ===========================================================================

#[test]
fn ex_03_every_threshold_in_the_produced_value_band() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in MODES {
            for seed in [0, 1, 7, 999, 1_000, 65_535] {
                // 0..=3000 covers every steady-state value of all three ops.
                for thr in 0..=3_000 {
                    h.assert_gotomach_sweep(args(40, seed, mode, thr));
                }
            }
        }
    });
}

#[test]
fn ex_04_every_threshold_around_the_first_step_values() {
    let mut h = harness();
    sweep(&mut h, |h| {
        // The very first produced value can be as large as 65535*3 = 196 605,
        // far outside the steady-state band; sweep its whole neighbourhood.
        for mode in [0, 1, 2] {
            for seed in [65_535, 65_534, 60_000, 1_000] {
                for thr in 0..=2_000 {
                    h.assert_gotomach_sweep(args(1, seed, mode, thr));
                }
                for &base in &[65_545, 131_070, 196_605] {
                    for d in -4..=4 {
                        h.assert_gotomach_sweep(args(1, seed, mode, base + d));
                        h.assert_gotomach_sweep(args(5, seed, mode, base + d));
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Exhaustive over small and mid-sized `iterations`
// ===========================================================================

#[test]
fn ex_05_every_iterations_up_to_1024() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in MODES {
            for seed in [0, 1, 65_535] {
                for &thr in &[c_int::MIN, 0, 1_000, 1_006, 2_998, c_int::MAX] {
                    for it in 0..=1_024 {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Exhaustive over the top of the `iterations` range — the saturation edge
// ===========================================================================

#[test]
fn ex_06_every_iterations_near_uint16_max() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in [0, 1, 2, 5] {
            for seed in [0, 1, 65_535] {
                for &thr in &[c_int::MAX, 2_998, 1_006, 0, c_int::MIN] {
                    for it in 65_400..=65_535 {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Exhaustive over the validity boundaries of both range-checked arguments
// ===========================================================================

#[test]
fn ex_07_every_value_across_both_validity_edges() {
    let mut h = harness();
    sweep(&mut h, |h| {
        for mode in MODES {
            for &thr in &[c_int::MIN, 0, 1_006, c_int::MAX] {
                // Around the `iterations` edges.
                for it in -8..=8 {
                    for seed in [0, 1, 65_535] {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
                for it in 65_528..=65_544 {
                    for seed in [0, 1, 65_535] {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
                // Around the `seed` edges.
                for seed in -8..=8 {
                    for it in [0, 1, 2, 64] {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
                for seed in 65_528..=65_544 {
                    for it in [0, 1, 2, 64] {
                        h.assert_gotomach_sweep(args(it, seed, mode, thr));
                    }
                }
            }
        }
    });
}

// ===========================================================================
// Exhaustive over the three leaf operations' whole reachable input domain
// ===========================================================================

#[test]
fn ex_08_ops_exhaustive_over_reachable_inputs() {
    let mut h = harness();
    // `gotomach` can only ever pass 0..=65535 (the seed) or 0..=999 (a folded
    // value), so enumerate that domain completely for all three exports.
    for v in 0..=65_535 {
        for which in Op::ALL {
            h.assert_op(which, v, 0, std::ptr::null_mut());
        }
    }
    // Plus the negative mirror and both overflow neighbourhoods, exhaustively.
    for v in -65_536..=0 {
        for which in Op::ALL {
            h.assert_op(which, v, 0, std::ptr::null_mut());
        }
    }
    for base in [c_int::MIN, c_int::MAX - 4_096, c_int::MAX / 2, c_int::MAX / 3] {
        for d in 0..4_096 {
            let v = base.wrapping_add(d);
            for which in Op::ALL {
                h.assert_op(which, v, 0, std::ptr::null_mut());
            }
        }
    }
}
