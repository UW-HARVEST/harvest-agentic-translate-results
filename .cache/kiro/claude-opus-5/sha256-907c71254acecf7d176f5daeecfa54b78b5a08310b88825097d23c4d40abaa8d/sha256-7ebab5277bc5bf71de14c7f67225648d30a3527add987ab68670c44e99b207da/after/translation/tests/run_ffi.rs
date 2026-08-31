//! Differential test of the lowest-level exported function, `void run(int)`.
//!
//! `run` prints `the_house`, adds a floor, prints, adds a bathroom, prints,
//! adds `extra_bedrooms` bedrooms, prints — four lines per call, against state
//! that carries over between calls.

mod common;

use common::check_run;
use std::ffi::c_int;

#[test]
fn run_matches_c() {
    // The very first call also pins down the initial `{2, 5, 2.5}` values and
    // the exact `%d` / `%.1f` formatting.
    let cases: &[c_int] = &[
        0,
        1,
        -1,
        5,
        -5,
        2,
        10,
        -10,
        100,
        -100,
        1_000,
        -1_000,
        123_456,
        -123_456,
        1_000_000_000,
        -1_000_000_000,
        7,
        -7,
        0,
        0,
    ];

    for (i, &v) in cases.iter().enumerate() {
        check_run(v, &format!("run(extra_bedrooms={v}) [call #{i}]"));
    }
}

/// `bedrooms += extra_bedrooms` overflows `int` for these inputs. Whatever the
/// C build does here is the reference behaviour the translation must reproduce.
#[test]
fn run_matches_c_at_int_extremes() {
    let cases: &[c_int] = &[
        c_int::MAX,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN,
        1,
        -1,
        c_int::MAX,
        c_int::MIN,
    ];

    for (i, &v) in cases.iter().enumerate() {
        check_run(v, &format!("run(extra_bedrooms={v}) [call #{i}]"));
    }
}

/// Many consecutive calls: `floors` climbs by one and `bathrooms` by 1.0 each
/// time, so this also checks that `%.1f` rounding stays in step as the double
/// grows.
#[test]
fn run_matches_c_over_many_calls() {
    for i in 0..250 {
        check_run(i % 3 - 1, &format!("run(extra_bedrooms={}) [iter {i}]", i % 3 - 1));
    }
}
