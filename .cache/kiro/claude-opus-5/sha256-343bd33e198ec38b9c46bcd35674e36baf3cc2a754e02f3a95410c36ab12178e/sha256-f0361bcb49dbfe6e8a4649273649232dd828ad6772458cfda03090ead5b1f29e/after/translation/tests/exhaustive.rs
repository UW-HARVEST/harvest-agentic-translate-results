//! Exhaustive differential sweeps over the reachable input space.
//!
//! These compare RETURN VALUES only (no stdout capture), which is what makes an
//! exhaustive sweep affordable: the stdout side is pinned separately, because
//! the log lines are a pure function of which branch is taken and every one of
//! the 10 log sites in the C source is covered byte-exactly by
//! `tests/symbols.rs::harness_stdout_capture_sees_the_exact_log_bytes` and by
//! the per-row stdout comparisons in `tests/configs.rs` / `tests/errors.rs`.

mod common;

use common::{libs, GotomachFn, OpFn, Silence};
use libloading::Symbol;

fn goto_syms() -> (Symbol<'static, GotomachFn>, Symbol<'static, GotomachFn>) {
    let l = libs();
    (
        unsafe { l.c.get(b"gotomach\0") }.unwrap(),
        unsafe { l.r.get(b"gotomach\0") }.unwrap(),
    )
}

/// Every valid `seed` (all 65536 of them), for each mode path and several
/// threshold regimes.
#[test]
fn exhaustive_all_valid_seeds() {
    let _quiet = Silence::new();
    let (c, r) = goto_syms();
    for mode in [0i32, 1, 2, 3, -1] {
        for th in [i32::MIN, 0, 700, 1500, 3000, i32::MAX] {
            for seed in 0..=65535i32 {
                let a = unsafe { c(3, seed, mode, th) };
                let b = unsafe { r(3, seed, mode, th) };
                assert_eq!(
                    a, b,
                    "gotomach(3, {seed}, {mode}, {th}): C={a} Rust={b}"
                );
            }
        }
    }
}

/// Every `iterations` value in `[0, 4096]` (contiguous), which walks the
/// `count`/`capacity` relationship one step at a time.
#[test]
fn exhaustive_contiguous_iteration_counts() {
    let _quiet = Silence::new();
    let (c, r) = goto_syms();
    for mode in [0i32, 1, 2, 9] {
        for (seed, th) in [(0, i32::MAX), (1, 1500), (999, 1000), (65535, i32::MIN), (7, 25)] {
            for it in 0..=4096i32 {
                let a = unsafe { c(it, seed, mode, th) };
                let b = unsafe { r(it, seed, mode, th) };
                assert_eq!(
                    a, b,
                    "gotomach({it}, {seed}, {mode}, {th}): C={a} Rust={b}"
                );
            }
        }
    }
}

/// Contiguous sweep across each range-check boundary, in both parameters at
/// once, so the short-circuit order is checked at every nearby value.
#[test]
fn exhaustive_range_check_boundaries() {
    let _quiet = Silence::new();
    let (c, r) = goto_syms();
    let edges = |x: i32| (x - 3)..=(x + 3);
    let mut vals: Vec<i32> = Vec::new();
    vals.extend(edges(0));
    vals.extend(edges(65535));
    vals.extend(edges(i32::MIN + 3));
    vals.extend(edges(i32::MAX - 3));
    for &it in &vals {
        for &sd in &vals {
            for mode in [0i32, 1, 2, 4, -4] {
                for th in [i32::MIN, 0, 1500, i32::MAX] {
                    let a = unsafe { c(it, sd, mode, th) };
                    let b = unsafe { r(it, sd, mode, th) };
                    assert_eq!(
                        a, b,
                        "gotomach({it}, {sd}, {mode}, {th}): C={a} Rust={b}"
                    );
                }
            }
        }
    }
}

/// Contiguous sweep of `threshold` across the whole range of values the three
/// operations can produce, which walks the strict `<` comparison one unit at a
/// time on both sides of every reachable produced value.
#[test]
fn exhaustive_contiguous_thresholds() {
    let _quiet = Silence::new();
    let (c, r) = goto_syms();
    for mode in [0i32, 1, 2, -2] {
        for seed in [0i32, 1, 7, 500, 999, 1000, 1001, 65535] {
            for th in -5..=3010i32 {
                let a = unsafe { c(24, seed, mode, th) };
                let b = unsafe { r(24, seed, mode, th) };
                assert_eq!(
                    a, b,
                    "gotomach(24, {seed}, {mode}, {th}): C={a} Rust={b}"
                );
            }
        }
    }
}

/// The three operation callbacks over a large contiguous span plus every
/// power-of-two boundary and both overflow edges.
#[test]
fn exhaustive_operation_callbacks() {
    let _quiet = Silence::new();
    let l = libs();
    for name in [
        &b"process_value\0"[..],
        &b"double_value\0"[..],
        &b"triple_value\0"[..],
    ] {
        let c: Symbol<OpFn> = unsafe { l.c.get(name) }.unwrap();
        let r: Symbol<OpFn> = unsafe { l.r.get(name) }.unwrap();
        let mut vals: Vec<i32> = (-70000..=70000).collect();
        for bit in 0..31 {
            let p = 1i32 << bit;
            vals.extend([p - 1, p, p + 1, -p - 1, -p, -p + 1]);
        }
        vals.extend([
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 2,
            i32::MAX - 2,
            i32::MAX - 1,
            i32::MAX,
            i32::MAX / 2,
            i32::MAX / 3,
            i32::MIN / 2,
            i32::MIN / 3,
        ]);
        for v in vals {
            let a = unsafe { c(v, 0, std::ptr::null_mut()) };
            let b = unsafe { r(v, 0, std::ptr::null_mut()) };
            assert_eq!(
                a,
                b,
                "{}({v}): C={a} Rust={b}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            );
        }
    }
}
