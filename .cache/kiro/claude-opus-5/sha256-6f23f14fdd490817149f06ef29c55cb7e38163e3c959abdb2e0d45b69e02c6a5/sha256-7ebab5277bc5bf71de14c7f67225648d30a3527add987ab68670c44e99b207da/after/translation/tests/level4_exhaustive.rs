//! Exhaustive sweeps over the constrained input dimensions.
//!
//! `seed` and `iterations` are both validated into `[0, UINT16_MAX]`, so the
//! reachable space for the interesting arguments is small enough to enumerate.

mod common;

use common::*;

fn every_valid_seed() {
    // 3 iterations is enough for the `op -> % 1000` feedback to matter, and
    // covers seeds above 1000 whose first result is not reduced yet.
    for mode in 0..3 {
        for seed in 0..=65535 {
            compare_gotomach(3, seed, mode, 1500);
        }
    }
}

fn every_valid_iteration_count_mode0() {
    sweep_iterations(0, 12345, 700);
}

fn every_valid_iteration_count_mode1() {
    sweep_iterations(1, 999, 1200);
}

fn every_valid_iteration_count_mode2() {
    sweep_iterations(2, 65535, 2000);
}

/// Every iteration count up to 2000 exactly, then a stride sweep up to the
/// `UINT16_MAX` limit (a full 0..=65535 sweep would be quadratic work).
fn sweep_iterations(mode: std::ffi::c_int, seed: std::ffi::c_int, threshold: std::ffi::c_int) {
    for it in 0..=2000 {
        compare_gotomach(it, seed, mode, threshold);
    }
    let mut it = 2001;
    while it <= 65535 {
        compare_gotomach(it, seed, mode, threshold);
        it += 379; // coprime-ish stride so residues vary
    }
    for &it in &[65530, 65531, 65532, 65533, 65534, 65535] {
        compare_gotomach(it, seed, mode, threshold);
    }
}

/// Single entry point: the stdout capture redirects the process-wide fd 1, so
/// no other libtest thread may be writing while a case runs.
#[test]
fn level4_exhaustive_all() {
    eprintln!("  case group: every_valid_seed");
    every_valid_seed();
    eprintln!("  case group: every_valid_iteration_count_mode0");
    every_valid_iteration_count_mode0();
    eprintln!("  case group: every_valid_iteration_count_mode1");
    every_valid_iteration_count_mode1();
    eprintln!("  case group: every_valid_iteration_count_mode2");
    every_valid_iteration_count_mode2();
}
