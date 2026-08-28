//! Level 4: `betagamma`, the sole entry point declared in `include/lib.h`.
//!
//! ## Why these tests fork
//!
//! `betagamma` folds `compute_hash` into its result, and `compute_hash` scores
//! the *relative addresses* returned by `malloc`/`calloc`. Because the two
//! blocks it allocates land in the same glibc tcache bin and are freed before
//! the next call, the addresses -- and therefore the return value -- depend on
//! the heap history of the whole process. The C reference is itself not
//! constant across repeated calls: `betagamma(1, 2, 3, 4)` called six times in
//! a fresh process yields `517 517 517 527 517 617`.
//!
//! Loading both libraries into one process is therefore not a valid comparison:
//! whichever one runs second sees a heap the other one disturbed. Instead each
//! implementation is driven in its own freshly-forked process over an identical
//! call sequence, and the full sequences of results are compared. That checks
//! the arithmetic *and* the allocation pattern -- a translation that allocated a
//! different number of chunks, in a different order, or in different size
//! classes would produce a different sequence and fail.

mod common;

use common::{BetagammaFn, Impl, pair};
use std::ffi::c_int;
use std::process::Command;

/// Which implementation a forked worker should exercise.
const IMPL_ENV: &str = "IT_BETAGAMMA_IMPL";

/// The call sequence, generated identically in the parent and in both workers.
///
/// Order matters: it is part of what is being compared.
fn call_sequence() -> Vec<(c_int, c_int, c_int, c_int)> {
    let mut cases: Vec<(c_int, c_int, c_int, c_int)> = Vec::new();

    // Every residue of `param1 % 10`, including the negative ones that make
    // `(param1 % 10) + 5` negative, sign-extend to a huge `size_t`, fail
    // `calloc` and take the `-1` early return.
    for a in -25i32..=25 {
        cases.push((a, 3, 5, 7));
        cases.push((a, -3, -5, -7));
        cases.push((a, 0, 0, 0));
    }

    // Dense small sweep over all four parameters.
    for a in -12i32..=12 {
        for b in [-7i32, -1, 0, 1, 9] {
            for c in [-6i32, 0, 2, 11] {
                for d in [-5i32, 0, 4, 13] {
                    cases.push((a, b, c, d));
                }
            }
        }
    }

    // Extremes: the C relies on wrap-around signed arithmetic throughout
    // (`flag_contribution * id`, `sum1 - sum2`, the running `result`).
    let vals = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_007,
        -65_537,
        -10,
        -9,
        -1,
        0,
        1,
        10,
        65_537,
        1_000_000_007,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &a in &vals {
        for &b in &vals {
            cases.push((a, b, 1, 2));
            cases.push((a, 1, b, 2));
            cases.push((a, 1, 2, b));
            cases.push((a, b, b, b));
        }
    }

    // Pseudo-random sweep (fixed seed, so any failure is reproducible).
    let mut s: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = || {
        // xorshift64*
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32 as i32
    };
    for _ in 0..4000 {
        cases.push((next(), next(), next(), next()));
    }

    // Repeat a handful of tuples back to back, which is what exposes the
    // allocator-dependent 10/20 and 100/200 contributions from `compute_hash`.
    for tuple in [(1, 2, 3, 4), (0, 0, 0, 0), (-4, 17, -9, 33), (7, -7, 7, -7)] {
        for _ in 0..64 {
            cases.push(tuple);
        }
    }

    cases
}

/// Worker body: load a single implementation and print one result per line.
///
/// This runs only in the forked child; in a normal `cargo test` run `IMPL_ENV`
/// is unset and the test is a no-op.
#[test]
fn betagamma_worker() {
    let which = match std::env::var(IMPL_ENV) {
        Ok(v) => v,
        Err(_) => return,
    };
    let p = pair_for(&which);
    let f: BetagammaFn = *p.betagamma();
    // Leading newline: libtest's `test betagamma_worker ... ` prefix is already
    // on the current line under `--nocapture`, and would otherwise swallow the
    // first result.
    let mut out = String::from("\n");
    for (a, b, c, d) in call_sequence() {
        out.push_str(&format!("R {}\n", unsafe { f(a, b, c, d) }));
    }
    print!("{out}");
}

/// Select one of the two libraries.
///
/// Both are loaded in every worker, in the same order, even though only one is
/// called: `betagamma`'s result depends on the heap state at the moment of the
/// call, so the workers must reach that point having done identical work. (A
/// worker that mapped only its own library measures the difference between the
/// two `dlopen`s rather than the difference between the two implementations.)
fn pair_for(which: &str) -> &'static Impl {
    let p = pair();
    match which {
        "c" => &p.c,
        "rust" => &p.rs,
        other => panic!("unknown implementation {other:?}"),
    }
}

/// Fork a worker for `which` and collect its result sequence.
fn run_worker(which: &str) -> Vec<i64> {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["--exact", "betagamma_worker", "--nocapture", "--test-threads=1"])
        .env(IMPL_ENV, which)
        .output()
        .unwrap_or_else(|e| panic!("failed to fork worker for {which}: {e}"));
    assert!(
        out.status.success(),
        "worker for {which} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let vals: Vec<i64> = text
        .lines()
        .filter_map(|l| l.strip_prefix("R "))
        .map(|v| v.trim().parse::<i64>().expect("worker printed a non-integer"))
        .collect();
    assert!(!vals.is_empty(), "worker for {which} produced no results");
    vals
}

/// The headline equivalence test: identical call sequence, one fresh process
/// per implementation, sequences must match element for element.
#[test]
fn betagamma_matches_c_over_full_sequence() {
    let cases = call_sequence();
    let c_vals = run_worker("c");
    let rs_vals = run_worker("rust");

    assert_eq!(
        c_vals.len(),
        cases.len(),
        "C worker returned {} results for {} cases",
        c_vals.len(),
        cases.len()
    );
    assert_eq!(
        rs_vals.len(),
        c_vals.len(),
        "worker result counts differ (C={}, Rust={})",
        c_vals.len(),
        rs_vals.len()
    );

    let mut mismatches = Vec::new();
    for (i, ((a, b, c, d), (cv, rv))) in cases
        .iter()
        .copied()
        .zip(c_vals.iter().copied().zip(rs_vals.iter().copied()))
        .enumerate()
    {
        if cv != rv {
            mismatches.push(format!(
                "  #{i}: betagamma({a}, {b}, {c}, {d}) -> C={cv} Rust={rv}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} calls mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The `-1` error path is address-independent, so it can be checked directly in
/// this process against both libraries.
#[test]
fn betagamma_error_path_returns_minus_one() {
    let p = pair();
    let c: BetagammaFn = *p.c.betagamma();
    let r: BetagammaFn = *p.rs.betagamma();
    for a in [-6i32, -7, -8, -9, -16, -17, -18, -19, -106, -999_999_999, i32::MIN + 1] {
        let cv = unsafe { c(a, 1, 2, 3) };
        let rv = unsafe { r(a, 1, 2, 3) };
        assert_eq!(cv, rv, "betagamma({a},1,2,3) mismatch");
        assert_eq!(cv, -1, "expected the C error path for param1={a}");
    }
}

/// Environment variable carrying the single tuple a cold-start worker must run.
const COLD_ENV: &str = "IT_BETAGAMMA_COLD";

/// Cold-start worker: one library, one call, pristine heap.
#[test]
fn betagamma_cold_worker() {
    let (Ok(which), Ok(args)) = (std::env::var(IMPL_ENV), std::env::var(COLD_ENV)) else {
        return;
    };
    let v: Vec<c_int> = args
        .split(',')
        .map(|s| s.parse().expect("bad tuple component"))
        .collect();
    assert_eq!(v.len(), 4);
    let f: BetagammaFn = *pair_for(&which).betagamma();
    println!("\nR {}", unsafe { f(v[0], v[1], v[2], v[3]) });
}

fn run_cold(which: &str, t: (c_int, c_int, c_int, c_int)) -> i64 {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args([
            "--exact",
            "betagamma_cold_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(IMPL_ENV, which)
        .env(COLD_ENV, format!("{},{},{},{}", t.0, t.1, t.2, t.3))
        .output()
        .unwrap_or_else(|e| panic!("failed to fork cold worker for {which}: {e}"));
    assert!(
        out.status.success(),
        "cold worker for {which} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("R "))
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or_else(|| panic!("cold worker for {which} printed no result"))
}

/// The first call in an otherwise untouched process -- the case an external
/// caller is most likely to hit -- must agree between the two libraries.
#[test]
fn betagamma_cold_start_matches_c() {
    let tuples = [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (5, 5, 5, 5),
        (9, 1, 1, 1),
        (10, -20, 30, -40),
        (-4, 17, -9, 33),
        (7, -7, 7, -7),
        (100, 200, 300, 400),
        (12345, -6789, 42, -42),
        (i32::MAX, i32::MIN, 1, -1),
        (i32::MIN, i32::MAX, -1, 1),
        (-6, 1, 2, 3),
        (1_000_000_007, 65_537, -65_537, 0),
        (3, 0, 0, 0),
    ];
    for t in tuples {
        let cv = run_cold("c", t);
        let rv = run_cold("rust", t);
        assert_eq!(
            cv, rv,
            "cold-start betagamma({}, {}, {}, {}): C={cv} Rust={rv}",
            t.0, t.1, t.2, t.3
        );
    }
}
