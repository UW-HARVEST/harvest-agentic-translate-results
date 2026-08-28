//! Level 2: the public `gotomach` entry point, exercising every validation
//! branch, every `switch (mode)` arm and the accumulation loop.

mod common;

use common::*;
use std::ffi::c_int;

fn invalid_iteration_count_branch() {
    for &it in &[-1, -2, -100, c_int::MIN, 65536, 65537, 1_000_000, c_int::MAX] {
        compare_gotomach(it, 0, 0, 0);
        compare_gotomach(it, 12345, 2, 500);
    }
}

fn invalid_seed_branch() {
    for &seed in &[-1, -2, -65535, c_int::MIN, 65536, 100_000, c_int::MAX] {
        compare_gotomach(0, seed, 0, 0);
        compare_gotomach(10, seed, 1, 100);
    }
}

fn all_mode_arms() {
    for &mode in &[0, 1, 2, 3, 4, -1, -2, 100, c_int::MAX, c_int::MIN] {
        for &it in &[0, 1, 2, 5] {
            compare_gotomach(it, 7, mode, 1000);
        }
    }
}

fn zero_iterations() {
    // capacity == 0 => malloc(0); the C loop never runs and the result is 0.
    for &mode in &[0, 1, 2, 9] {
        for &seed in &[0, 1, 65535] {
            for &th in &[c_int::MIN, -1, 0, 1, 1000, c_int::MAX] {
                compare_gotomach(0, seed, mode, th);
            }
        }
    }
}

fn threshold_boundaries() {
    // threshold decides whether each produced value lands in `results`.
    for &mode in &[0, 1, 2] {
        for &seed in &[0, 1, 9, 100, 333, 500, 999, 1000, 65535] {
            for &th in &[
                c_int::MIN,
                -1000,
                -1,
                0,
                1,
                9,
                10,
                11,
                100,
                999,
                1000,
                1001,
                1998,
                2000,
                2997,
                3000,
                196_605,
                196_606,
                c_int::MAX,
            ] {
                compare_gotomach(8, seed, mode, th);
            }
        }
    }
}

fn seed_boundaries() {
    for &seed in &[0, 1, 2, 333, 334, 500, 999, 1000, 1001, 32767, 65534, 65535] {
        for &mode in &[0, 1, 2, 7] {
            compare_gotomach(16, seed, mode, 100_000);
            compare_gotomach(16, seed, mode, 500);
        }
    }
}

fn small_iteration_counts() {
    for it in 0..40 {
        for &mode in &[0, 1, 2] {
            compare_gotomach(it, 12345, mode, 2000);
        }
    }
}

fn cycle_detection_long_runs() {
    // The value sequence is `v -> op(v) % 1000`, which cycles quickly; run long
    // enough that the accumulated sum depends on the exact cycle behaviour.
    for &it in &[100, 999, 1000, 1024, 4096, 12345, 65534, 65535] {
        for &mode in &[0, 1, 2] {
            compare_gotomach(it, 1, mode, 100_000);
            compare_gotomach(it, 65535, mode, 777);
        }
    }
}

fn max_capacity() {
    // iterations == UINT16_MAX with a threshold that stores every value is the
    // closest the C gets to its `count >= UINT16_MAX` guard.
    for &mode in &[0, 1, 2] {
        compare_gotomach(65535, 0, mode, c_int::MAX);
        compare_gotomach(65535, 65535, mode, c_int::MAX);
    }
}

fn randomised_cross_check() {
    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..400 {
        let it = rng.range(-4, 70) as c_int;
        let seed = rng.range(-4, 65540) as c_int;
        let mode = rng.range(-3, 6) as c_int;
        let threshold = rng.range(-2000, 4000) as c_int;
        compare_gotomach(it, seed, mode, threshold);
    }
}

fn randomised_extreme_cross_check() {
    let mut rng = Rng::new(0xABCD_EF01);
    for _ in 0..120 {
        let it = match rng.range(0, 3) {
            0 => rng.range(0, 65535) as c_int,
            1 => rng.next_u32() as c_int,
            2 => rng.range(65530, 65540) as c_int,
            _ => rng.range(-10, 10) as c_int,
        };
        let seed = match rng.range(0, 2) {
            0 => rng.range(0, 65535) as c_int,
            1 => rng.next_u32() as c_int,
            _ => rng.range(65530, 65540) as c_int,
        };
        let mode = rng.next_u32() as c_int;
        let threshold = rng.next_u32() as c_int;
        compare_gotomach(it, seed, mode, threshold);
    }
}

/// Single entry point: the stdout capture redirects the process-wide fd 1, so
/// no other libtest thread may be writing while a case runs.
#[test]
fn level2_gotomach_all() {
    eprintln!("  case group: invalid_iteration_count_branch");
    invalid_iteration_count_branch();
    eprintln!("  case group: invalid_seed_branch");
    invalid_seed_branch();
    eprintln!("  case group: all_mode_arms");
    all_mode_arms();
    eprintln!("  case group: zero_iterations");
    zero_iterations();
    eprintln!("  case group: threshold_boundaries");
    threshold_boundaries();
    eprintln!("  case group: seed_boundaries");
    seed_boundaries();
    eprintln!("  case group: small_iteration_counts");
    small_iteration_counts();
    eprintln!("  case group: cycle_detection_long_runs");
    cycle_detection_long_runs();
    eprintln!("  case group: max_capacity");
    max_capacity();
    eprintln!("  case group: randomised_cross_check");
    randomised_cross_check();
    eprintln!("  case group: randomised_extreme_cross_check");
    randomised_extreme_cross_check();
}
