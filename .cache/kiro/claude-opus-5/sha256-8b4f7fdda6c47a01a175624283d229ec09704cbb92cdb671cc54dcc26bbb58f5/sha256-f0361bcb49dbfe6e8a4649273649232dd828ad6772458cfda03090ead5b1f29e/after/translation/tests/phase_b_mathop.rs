//! Phase B (continued) — `mathop`, the one-shot wrapper declared in
//! `c_src/include/lib.h`. Covers `CONFIGS.md` rows 25-30.
//!
//! `mathop` has two observable outputs: the returned `int` and four `printf`
//! lines. Both are compared byte-for-byte.
//!
//! Everything lives in ONE `#[test]` fn on purpose: each `.so` keeps its own
//! function-local `static history_count`, so the two sides only stay in step if
//! C and Rust are invoked the same number of times, and fd-1 redirection is
//! process-global so it must not race with another test thread.

mod common;

use common::*;
use std::ffi::c_int;

/// `INT_MIN / -1` (and `%`) raise SIGFPE in the C code — undefined behaviour
/// that the Rust side reproduces faithfully. It is asserted out-of-process in
/// Phase C, so in-process callers must avoid it.
///
/// Only the *first* inner computation can reach it: the second one uses
/// `b = param4`, and `param4 == -1` forces `second_op == 1` (`add`), which never
/// divides.
fn traps(param1: c_int, param2: c_int, param3: c_int) -> bool {
    let first_op = param3.wrapping_rem(5).wrapping_add(1);
    param1 == c_int::MIN && param2 == -1 && (first_op == 4 || first_op == 5)
}

/// One paired invocation: run C's `mathop`, then Rust's, on identical args and
/// compare return value plus captured stdout.
fn diff_mathop(label: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (Vec<u8>, Vec<u8>) {
    assert!(!traps(p1, p2, p3), "{label}: caller must filter trapping inputs");
    let (cv, cout) = capture_stdout(|| unsafe { (c().mathop)(p1, p2, p3, p4) });
    let (rv, rout) = capture_stdout(|| unsafe { (r().mathop)(p1, p2, p3, p4) });
    assert_eq!(
        cv, rv,
        "{label}: mathop({p1}, {p2}, {p3}, {p4}) return value C={cv} RUST={rv}\n\
         C stdout:    {}\n RUST stdout: {}",
        bytes_dbg(&cout),
        bytes_dbg(&rout)
    );
    assert_eq!(
        cout,
        rout,
        "{label}: mathop({p1}, {p2}, {p3}, {p4}) stdout differs\n\
         C:    {}\n RUST: {}",
        bytes_dbg(&cout),
        bytes_dbg(&rout)
    );
    assert!(
        cout.starts_with(b"Computation performed at timestamp: "),
        "{label}: unexpected stdout shape: {}",
        bytes_dbg(&cout)
    );
    (cout, rout)
}

fn history_entries(out: &[u8]) -> i64 {
    let s = String::from_utf8_lossy(out);
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("History entries: ") {
            return rest.trim().parse().expect("parse history entries");
        }
    }
    panic!("no 'History entries:' line in {}", bytes_dbg(out));
}

#[test]
fn phase_b_mathop_all_rows() {
    let _guard = CAPTURE_LOCK.lock().unwrap();

    // ---------------------------------------------------------------------
    // Row 29 — axis D: the process-wide history counter's trajectory.
    // MUST run first: both libraries' statics are still at 0 here.
    // ---------------------------------------------------------------------
    let expected_traj = [2i64, 4, 6, 8, 10, 10, 10, 10];
    for (i, want) in expected_traj.iter().enumerate() {
        let (cout, rout) = diff_mathop(
            &format!("row29 call {i}"),
            49 + i as c_int,
            i as c_int * 3 + 1,
            i as c_int,
            i as c_int + 2,
        );
        assert_eq!(
            history_entries(&cout),
            *want,
            "row29: C history trajectory at call {i}"
        );
        assert_eq!(
            history_entries(&rout),
            *want,
            "row29: RUST history trajectory at call {i}"
        );
    }

    // ---------------------------------------------------------------------
    // Row 25 — axis E = valid (param1 % 128 in 49..=53) x axis F.
    // ---------------------------------------------------------------------
    let mut rng = Rng::new(0x2525);
    for vc in 49..=53 {
        for k in 0..128 {
            // param1 % 128 == vc for these, covering both signs of param1.
            let p1 = vc + 128 * k;
            assert!((49..=53).contains(&(p1 % 128)), "row25 setup");
            for r3 in 0..5 {
                let p3 = r3 + 5 * (k % 7);
                let p4 = rng.next_i32_mixed();
                if traps(p1, rng.next_i32(), p3) {
                    continue;
                }
                let p2 = {
                    let mut b = rng.next_i32_mixed();
                    if traps(p1, b, p3) {
                        b = 3;
                    }
                    b
                };
                diff_mathop("row25", p1, p2, p3, p4);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Row 26 — axis E = invalid x the full 5x5 (param3 mod 5, param4 mod 5).
    // ---------------------------------------------------------------------
    let mut rng = Rng::new(0x2626);
    for r3 in 0..5 {
        for r4 in 0..5 {
            for _ in 0..30 {
                // param1 % 128 outside 49..=53 => is_valid == false.
                let mut p1 = rng.next_i32_mixed();
                if (49..=53).contains(&p1.wrapping_rem(128)) {
                    p1 = p1.wrapping_add(1);
                }
                assert!(!(49..=53).contains(&p1.wrapping_rem(128)), "row26 setup");
                let p3 = r3 + 5 * (rng.range(0, 1000) as c_int);
                let p4 = r4 + 5 * (rng.range(0, 1000) as c_int);
                let mut p2 = rng.next_i32_mixed();
                if traps(p1, p2, p3) {
                    p2 = 3;
                }
                diff_mathop("row26", p1, p2, p3, p4);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Row 27 — negative param3/param4 => out-of-range Operation from `%`.
    // ---------------------------------------------------------------------
    let mut rng = Rng::new(0x2727);
    for p3 in -12..=0 {
        for p4 in -12..=0 {
            for _ in 0..4 {
                let p1 = rng.next_i32_mixed();
                let mut p2 = rng.next_i32_mixed();
                if traps(p1, p2, p3) {
                    p2 = 3;
                }
                diff_mathop("row27", p1, p2, p3, p4);
            }
        }
    }
    for &p3 in &[c_int::MIN, c_int::MIN + 1, -1, -5, -2147483645] {
        for &p4 in &[c_int::MIN, c_int::MIN + 1, -1, -5, -6, 0] {
            let mut p2 = -1;
            if traps(7, p2, p3) {
                p2 = 3;
            }
            diff_mathop("row27b", 7, p2, p3, p4);
        }
    }

    // ---------------------------------------------------------------------
    // Row 28 — axis B boundary values as parameters.
    // ---------------------------------------------------------------------
    for &p1 in BOUNDARIES.iter() {
        for &p2 in BOUNDARIES.iter() {
            for &p3 in BOUNDARIES.iter() {
                for &p4 in &[c_int::MIN, -1, 0, 1, c_int::MAX] {
                    if traps(p1, p2, p3) {
                        continue;
                    }
                    diff_mathop("row28", p1, p2, p3, p4);
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Row 30 — fully randomized quadruples.
    // ---------------------------------------------------------------------
    let mut rng = Rng::new(0x3030);
    let mut done = 0;
    while done < 400 {
        let p1 = rng.next_i32_mixed();
        let p2 = rng.next_i32_mixed();
        let p3 = rng.next_i32_mixed();
        let p4 = rng.next_i32_mixed();
        if traps(p1, p2, p3) {
            continue;
        }
        diff_mathop("row30", p1, p2, p3, p4);
        done += 1;
    }
}
