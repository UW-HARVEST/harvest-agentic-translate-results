//! Level 3: the public entry point `maxnmin`, swept exhaustively over
//! interesting parameter values plus a large randomised sample.
mod harness;

use harness::{impls, Api};
use std::ffi::c_int;

fn edge_values() -> Vec<c_int> {
    vec![
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 2,
        -2_000_000_000,
        -1_000_000,
        -100,
        -12,
        -7,
        -6,
        -5,
        -3,
        -2,
        -1, // param3 == -1 makes the final division divide by zero
        0,
        1,
        2,
        3,
        5,
        6,
        7,
        12,
        100,
        1_000_000,
        2_000_000_000,
        c_int::MAX - 1,
        c_int::MAX,
    ]
}

fn check(i: &harness::Impls, a: c_int, b: c_int, c: c_int, d: c_int) {
    let expected = unsafe { (i.c.maxnmin)(a, b, c, d) };
    for r in &i.rust {
        let got = unsafe { (r.maxnmin)(a, b, c, d) };
        assert_eq!(
            expected, got,
            "maxnmin({a}, {b}, {c}, {d}) C={expected} {}={got}",
            r.label
        );
    }
}

/// Every combination of the interesting values: covers negative `%` truncation
/// for the node/parent selection, `param3 == -1` (division by zero producing
/// inf/NaN), and `param3 == INT_MAX` (the `param3 + 1` wrap).
#[test]
fn maxnmin_edge_value_cartesian() {
    let i = impls();
    let v = edge_values();
    for &a in &v {
        for &b in &v {
            for &c in &v {
                for &d in &v {
                    check(&i, a, b, c, d);
                }
            }
        }
    }
}

/// Dense sweep of small values, where the `% 6` / `% 3` selectors change on
/// every step.
#[test]
fn maxnmin_small_dense_sweep() {
    let i = impls();
    for a in -13..=13 {
        for b in -13..=13 {
            for c in -13..=13 {
                for d in -13..=13 {
                    check(&i, a, b, c, d);
                }
            }
        }
    }
}

/// Large uniform random sample across the whole i32 range.
#[test]
fn maxnmin_random_sample() {
    let i = impls();
    let mut x: u64 = 0x5eed_1234_abcd_9876;
    let mut next = || {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (x >> 32) as u32 as c_int
    };
    for _ in 0..150_000 {
        let a = next();
        let b = next();
        let c = next();
        let d = next();
        check(&i, a, b, c, d);
    }
}

/// `param1 + param2` and `param3 + 1` overflow paths, and pairs that sum to
/// exactly zero while `param3 == -1` (yielding 0.0/0.0 = NaN).
#[test]
fn maxnmin_overflow_and_nan_paths() {
    let i = impls();
    let pairs = [
        (c_int::MAX, c_int::MAX),
        (c_int::MAX, 1),
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, -1),
        (c_int::MAX, c_int::MIN),
        (1_500_000_000, 1_500_000_000),
        (-1_500_000_000, -1_500_000_000),
        (5, -5),
        (0, 0),
        (-7, 7),
        (c_int::MIN, 0),
    ];
    for (a, b) in pairs {
        for c in [-1, 0, 1, -2, c_int::MAX, c_int::MIN, c_int::MIN + 1, 2] {
            for d in [0, 1, -1, 2, 3, c_int::MAX, c_int::MIN, 100_000] {
                check(&i, a, b, c, d);
            }
        }
    }
}

/// `maxnmin` must be self-consistent regardless of the state left behind by
/// previous calls, and must leave the node table in an identical condition.
#[test]
fn maxnmin_is_idempotent_across_state() {
    let i = impls();
    let seq = [
        (0, 0, 0, 0),
        (5, 3, 2, 1),
        (-1, -1, -1, -1),
        (c_int::MAX, c_int::MIN, -1, 7),
        (5, 3, 2, 1),
        (0, 0, 0, 0),
    ];
    for (a, b, c, d) in seq {
        check(&i, a, b, c, d);
        let table = |api: &Api| unsafe {
            let base = (api.find_node_by_id)(1) as *const u8;
            assert!(!base.is_null());
            std::slice::from_raw_parts(base, 6 * std::mem::size_of::<harness::Node>()).to_vec()
        };
        let cbytes = table(&i.c);
        for r in &i.rust {
            assert_eq!(
                cbytes,
                table(r),
                "node table after maxnmin({a},{b},{c},{d}) differs in {}",
                r.label
            );
        }
    }
}
