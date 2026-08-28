//! Extra robustness differentials that fall outside the `CONFIGS.md` /
//! `ERRORS.md` grids but are real properties of the C library:
//!
//!   * reentrancy — the C uses only locals plus `malloc`, so `gotomach` is
//!     thread-safe; the Rust translation must be too;
//!   * memory hygiene — the C `free`s both allocations on every exit path
//!     (including all six error paths), so repeated calls must not grow RSS;
//!     a leaked `Box`/`Vec` in the Rust would show up here and nowhere else.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// Reentrancy
// ===========================================================================

#[test]
fn r1_gotomach_is_reentrant_from_many_threads() {
    let mut h = harness();
    // Function pointers are `Send`; the two libraries stay loaded for the
    // process lifetime, so the threads can safely use them.
    let cf = h.c.gotomach;
    let rf = h.r.gotomach;

    // Reference answers computed serially first.
    let cases: Vec<Args> = {
        let mut rng = Rng::new(SEED ^ 0xC0FFEE);
        (0..256)
            .map(|_| {
                args(
                    rng.range(0, 2_000),
                    rng.range(0, 65_535),
                    rng.i32_interesting(),
                    rng.i32_interesting(),
                )
            })
            .collect()
    };
    let expected: Vec<c_int> = sweep(&mut h, |h| {
        cases.iter().map(|a| h.assert_gotomach_sweep(*a)).collect()
    });

    // Now hammer both libraries concurrently; every thread must reproduce the
    // serial answers, and C and Rust must still agree call for call.
    sweep(&mut h, |_h| {
        std::thread::scope(|s| {
            for t in 0..8u32 {
                let cases = &cases;
                let expected = &expected;
                s.spawn(move || {
                    for round in 0..6 {
                        for (i, a) in cases.iter().enumerate() {
                            let rc = unsafe { cf(a.iterations, a.seed, a.mode, a.threshold) };
                            let rr = unsafe { rf(a.iterations, a.seed, a.mode, a.threshold) };
                            assert_eq!(
                                rc, rr,
                                "thread {t} round {round}: C/Rust diverged for {a}"
                            );
                            assert_eq!(
                                rc, expected[i],
                                "thread {t} round {round}: {a} is not reentrant"
                            );
                        }
                    }
                });
            }
        });
    });
}

#[test]
fn r2_ops_are_reentrant_from_many_threads() {
    let h = harness();
    let cops = [h.c.process_value, h.c.double_value, h.c.triple_value];
    let rops = [h.r.process_value, h.r.double_value, h.r.triple_value];
    std::thread::scope(|s| {
        for t in 0..8u32 {
            s.spawn(move || {
                let mut rng = Rng::new(SEED ^ (0x9E37 + t as u64));
                for _ in 0..50_000 {
                    let v = rng.i32_interesting();
                    let p = rng.i32_interesting();
                    for k in 0..3 {
                        let rc = unsafe { cops[k](v, p, std::ptr::null_mut()) };
                        let rr = unsafe { rops[k](v, p, std::ptr::null_mut()) };
                        assert_eq!(rc, rr, "thread {t}: op[{k}]({v}, {p}, NULL) diverged");
                    }
                }
            });
        }
    });
}

// ===========================================================================
// Memory hygiene
// ===========================================================================

/// Resident set size in pages, from `/proc/self/statm`.
fn rss_pages() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("/proc/self/statm");
    s.split_whitespace()
        .nth(1)
        .and_then(|x| x.parse().ok())
        .expect("statm rss field")
}

/// Runs `f` `n` times and returns the RSS growth in pages, discarding an
/// initial warm-up so first-touch page faults and allocator arena setup are not
/// counted as a leak.
fn rss_growth(warmup: usize, n: usize, mut f: impl FnMut(usize)) -> i64 {
    for i in 0..warmup {
        f(i);
    }
    let before = rss_pages();
    for i in 0..n {
        f(i);
    }
    let after = rss_pages();
    after as i64 - before as i64
}

#[test]
fn r3_neither_implementation_leaks_on_the_success_path() {
    let mut h = harness();
    let cf = h.c.gotomach;
    let rf = h.r.gotomach;
    // 65535 iterations => two ~256 KiB allocations per call (64 pages each), so
    // a per-call leak of even one allocation would add ~64 pages/call.
    let n = 400;
    let (c_growth, r_growth) = sweep(&mut h, |_h| {
        let c = rss_growth(50, n, |i| {
            unsafe { cf(65_535, (i % 65_536) as c_int, (i % 4) as c_int, c_int::MAX) };
        });
        let r = rss_growth(50, n, |i| {
            unsafe { rf(65_535, (i % 65_536) as c_int, (i % 4) as c_int, c_int::MAX) };
        });
        (c, r)
    });
    eprintln!("RSS growth over {n} max-size calls: C {c_growth} pages, Rust {r_growth} pages");
    // The C frees both buffers every call, so growth is ~0. Allow generous
    // slack for allocator noise, but far below the 64 pages/call a real leak
    // would cost (400 * 64 = 25 600 pages).
    let budget = 2_000;
    assert!(
        c_growth < budget,
        "C grew by {c_growth} pages — measurement is unreliable"
    );
    assert!(
        r_growth < budget,
        "Rust leaked: RSS grew by {r_growth} pages over {n} calls (C grew {c_growth})"
    );
}

#[test]
fn r4_neither_implementation_leaks_on_the_error_paths() {
    let mut h = harness();
    let cf = h.c.gotomach;
    let rf = h.r.gotomach;
    // -1 and -2 reach `cleanup:` with both pointers NULL; -2 with a large
    // valid `iterations` still allocates nothing, because the seed check comes
    // first. Mix them, plus success paths, to exercise every cleanup arm.
    let cases: [(c_int, c_int); 6] = [
        (-1, 0),      // -1, nothing allocated
        (65_536, 0),  // -1, nothing allocated
        (65_535, -1), // -2, nothing allocated
        (65_535, 65_536),
        (65_535, 0), // success, both buffers allocated and freed
        (0, 0),      // success, malloc(0) x2 allocated and freed
    ];
    let n = 6_000;
    let (c_growth, r_growth) = sweep(&mut h, |_h| {
        let c = rss_growth(600, n, |i| {
            let (it, sd) = cases[i % cases.len()];
            unsafe { cf(it, sd, (i % 5) as c_int, c_int::MAX) };
        });
        let r = rss_growth(600, n, |i| {
            let (it, sd) = cases[i % cases.len()];
            unsafe { rf(it, sd, (i % 5) as c_int, c_int::MAX) };
        });
        (c, r)
    });
    eprintln!("RSS growth over {n} mixed calls: C {c_growth} pages, Rust {r_growth} pages");
    let budget = 2_000;
    assert!(c_growth < budget, "C grew by {c_growth} pages");
    assert!(
        r_growth < budget,
        "Rust leaked on an error path: RSS grew by {r_growth} pages (C grew {c_growth})"
    );
}
