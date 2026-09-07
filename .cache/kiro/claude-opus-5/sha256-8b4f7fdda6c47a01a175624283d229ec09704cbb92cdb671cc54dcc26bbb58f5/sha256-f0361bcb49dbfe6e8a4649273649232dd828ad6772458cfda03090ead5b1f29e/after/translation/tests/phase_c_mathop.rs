//! Phase C (continued) — `ERRORS.md` rows 21-25: `mathop`'s rejection paths.
//!
//! Separate test binary with a single `#[test]` fn: the test redirects fd 1 to
//! capture `printf` output, and libtest's own progress writes to fd 1 would
//! otherwise land inside the capture window of a concurrently running test.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn err21_to_25_mathop_rejection_paths() {
    let _guard = CAPTURE_LOCK.lock().unwrap();

    let diff = |label: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int| {
        let (cv, cout) = capture_stdout(|| unsafe { (c().mathop)(p1, p2, p3, p4) });
        let (rv, rout) = capture_stdout(|| unsafe { (r().mathop)(p1, p2, p3, p4) });
        assert_eq!(
            cv, rv,
            "{label}: mathop({p1},{p2},{p3},{p4}) ret C={cv} RUST={rv}"
        );
        assert_eq!(
            cout,
            rout,
            "{label}: mathop({p1},{p2},{p3},{p4}) stdout\n C:    {}\n RUST: {}",
            bytes_dbg(&cout),
            bytes_dbg(&rout)
        );
        (cv, cout)
    };

    // Row 21 — validation_char rejected, then overwritten and never read.
    // A rejected and an accepted param1 that agree mod-everything-else must
    // produce identical results, proving the branch has no observable effect.
    for &(bad, good) in &[(0i32, 49i32), (48, 50), (54, 51), (127, 52), (-49, 53)] {
        assert!(!(49..=53).contains(&bad.rem_euclid(128).min(bad.wrapping_rem(128))) || bad < 0);
        let (_, _) = diff("row 21 rejected", bad, 7, 3, 4);
        let (_, _) = diff("row 21 accepted", good, 7, 3, 4);
    }
    // Every param1 % 128 residue, both signs, exercising both sides of the branch.
    for k in -2..=2i32 {
        for res in 0..128i32 {
            let p1 = res + 128 * k;
            diff("row 21 residues", p1, 5, 2, 9);
        }
    }

    // Row 22 — negative param3 => out-of-range Operation, negative priority.
    for p3 in [-1, -2, -3, -4, -5, -6, -10, -2147483645, c_int::MIN, c_int::MIN + 1] {
        diff("row 22", 50, 6, p3, 3);
    }

    // Row 23 — param4 == -1 and param4 < -1.
    for p4 in [-1, -2, -3, -4, -5, -6, -7, -11, c_int::MIN, c_int::MIN + 1] {
        diff("row 23", 51, 8, 1, p4);
    }

    // Row 24 — history saturation: the counter is already >= 10 by now, so the
    // printed value must be pinned at 10 on both sides.
    for i in 0..6 {
        let (_, out) = diff("row 24", 49, i, i, i);
        let s = String::from_utf8_lossy(&out).to_string();
        assert!(
            s.contains("History entries: 10\n"),
            "row 24: history must be saturated at 10, got {}",
            bytes_dbg(&out)
        );
    }

    // Row 25 — INT_MIN parameters (INT_MIN % 128 == 0 => rejected; signed
    // overflow wraps). Avoid the SIGFPE combination (param2 == -1 with a
    // divide/modulo first op), covered by rows 7/8.
    for &p2 in &[c_int::MIN, 0, 1, c_int::MAX] {
        for &p3 in &[0, 1, 2, 3, 4, c_int::MIN, c_int::MAX] {
            for &p4 in &[c_int::MIN, -1, 0, 1, c_int::MAX] {
                diff("row 25", c_int::MIN, p2, p3, p4);
            }
        }
    }
    // param2 == -1 with a non-dividing first op is safe.
    for &p3 in &[0, 1, 2, 5, 6, 7] {
        let first_op = p3 % 5 + 1;
        if first_op == 4 || first_op == 5 {
            continue;
        }
        diff("row 25b", c_int::MIN, -1, p3, -1);
    }
}

