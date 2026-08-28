//! Verifies the exact stdout bytes for each log branch, so the tests cannot
//! pass by both sides being silent.

mod common;

use common::*;
use std::ffi::c_int;

fn run(lib_is_rust: bool, it: c_int, seed: c_int, mode: c_int, th: c_int) -> (c_int, String) {
    let l = libs();
    let f = gotomach(if lib_is_rust { &l.rust } else { &l.c });
    let (v, out) = capture_stdout(|| unsafe { f(it, seed, mode, th) });
    (v, String::from_utf8(out).expect("log output is UTF-8"))
}

fn assert_both(it: c_int, seed: c_int, mode: c_int, th: c_int, expected: &str, ret: c_int) {
    let (cv, cout) = run(false, it, seed, mode, th);
    let (rv, rout) = run(true, it, seed, mode, th);
    assert_eq!(cout, expected, "C stdout differs from the expected literal");
    assert_eq!(rout, expected, "Rust stdout differs from the expected literal");
    assert_eq!(cv, ret, "C return value differs from expectation");
    assert_eq!(rv, ret, "Rust return value differs from expectation");
}

const START: &str = "[INFO] Starting gotomach function\n";

fn log_invalid_iterations() {
    assert_both(
        -1,
        0,
        0,
        0,
        &format!("{START}[ERROR] Invalid iteration count\n"),
        -1,
    );
    assert_both(
        65536,
        0,
        0,
        0,
        &format!("{START}[ERROR] Invalid iteration count\n"),
        -1,
    );
}

fn log_invalid_seed() {
    assert_both(1, -1, 0, 0, &format!("{START}[ERROR] Invalid seed value\n"), -2);
    assert_both(
        1,
        65536,
        0,
        0,
        &format!("{START}[ERROR] Invalid seed value\n"),
        -2,
    );
}

fn log_invalid_mode_warning() {
    // mode 3 falls into `default:` and logs a warning before succeeding.
    assert_both(
        1,
        5,
        3,
        1000,
        &format!("{START}[WARNING] Invalid mode, using default\n[INFO] Processing completed successfully\n"),
        15,
    );
}

fn log_success_path() {
    // mode 1 (double_value), seed 5, 1 iteration, threshold above the value.
    assert_both(
        1,
        5,
        1,
        1000,
        &format!("{START}[INFO] Processing completed successfully\n"),
        10,
    );
    // Same, but the value is filtered out by the threshold => sum stays 0.
    assert_both(
        1,
        5,
        1,
        10,
        &format!("{START}[INFO] Processing completed successfully\n"),
        0,
    );
}

fn log_max_count_warning() {
    // threshold == INT_MAX stores every value, so `count` reaches UINT16_MAX on
    // the last iteration and the "Reached maximum count" branch fires.
    let (_, cout) = run(false, 65535, 0, 0, c_int::MAX);
    let (_, rout) = run(true, 65535, 0, 0, c_int::MAX);
    assert!(
        cout.contains("[WARNING] Reached maximum count"),
        "expected the C to hit the max-count branch, got: {cout}"
    );
    assert_eq!(cout, rout);
}

/// Single entry point: the stdout capture redirects the process-wide fd 1, so
/// no other libtest thread may be writing while a case runs.
#[test]
fn level3_logs_all() {
    eprintln!("  case group: log_invalid_iterations");
    log_invalid_iterations();
    eprintln!("  case group: log_invalid_seed");
    log_invalid_seed();
    eprintln!("  case group: log_invalid_mode_warning");
    log_invalid_mode_warning();
    eprintln!("  case group: log_success_path");
    log_success_path();
    eprintln!("  case group: log_max_count_warning");
    log_max_count_warning();
}
