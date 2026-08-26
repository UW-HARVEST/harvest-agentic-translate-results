// Phase B — valid-path differential tests for the public `mathop` entry point.
// CONFIGS.md rows C27 .. C34.
//
// `mathop` keeps `static` state (`computation_history`, `history_count`) inside
// each shared object, and it prints four lines to stdout. Both libraries are
// therefore driven strictly in lockstep from one single `#[test]` (so the test
// process starts with both sets of statics fresh), and stdout is captured by
// redirecting fd 1 — libc `printf` bypasses the Rust test harness's capture.

mod common;

use common::*;
use std::ffi::c_int;

/// One lockstep `mathop` call pair: same arguments, both return values and both
/// stdout transcripts compared.
fn diff_mathop(p1: c_int, p2: c_int, p3: c_int, p4: c_int, ctx: &str) -> Vec<u8> {
    let (c, r) = both();
    assert!(
        !mathop_is_ub(p1, p2, p3, p4),
        "{ctx}: refusing to call mathop({p1},{p2},{p3},{p4}) — it reaches INT_MIN/-1 \
         signed-division UB (ERRORS.md E21), which SIGFPEs in the C reference build"
    );
    let _g = serial();

    let mut cv: c_int = 0;
    let cout = capture_stdout(|| cv = unsafe { (c.mathop)(p1, p2, p3, p4) });
    let mut rv: c_int = 0;
    let rout = capture_stdout(|| rv = unsafe { (r.mathop)(p1, p2, p3, p4) });

    assert_eq!(
        cv, rv,
        "{ctx}: mathop({p1},{p2},{p3},{p4}) returned C={cv} Rust={rv}"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{ctx}: mathop({p1},{p2},{p3},{p4}) stdout differs"
    );
    assert_eq!(
        cout, rout,
        "{ctx}: mathop({p1},{p2},{p3},{p4}) stdout bytes differ"
    );
    // Sanity: the transcript really is the four expected lines.
    let text = String::from_utf8_lossy(&cout).to_string();
    assert_eq!(
        text.lines().count(),
        4,
        "{ctx}: unexpected transcript {text:?}"
    );
    assert!(text.starts_with("Computation performed at timestamp: "), "{text:?}");
    assert!(text.contains("\nOperation priority: "), "{text:?}");
    assert!(text.contains("\nHistory entries: "), "{text:?}");
    assert!(text.contains("\nFinal result: "), "{text:?}");
    assert!(
        text.ends_with(&format!("Final result: {cv}\n")),
        "{ctx}: printed result must equal the returned one: {text:?}"
    );
    cout
}

fn field(transcript: &[u8], label: &str) -> i64 {
    let text = String::from_utf8_lossy(transcript).to_string();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(label) {
            return rest.trim().parse::<i64>().unwrap_or_else(|e| {
                panic!("cannot parse {label:?} from {line:?}: {e}")
            });
        }
    }
    panic!("label {label:?} not found in {text:?}");
}

/// `param1` values whose `(char)(param1 % 128)` *is* a valid '1'..'5'.
const VALID_CHARS: [c_int; 10] = [49, 50, 51, 52, 53, 177, 178, 179, 180, 181];
/// `param1` values whose validation char is rejected (ERRORS.md E15).
const INVALID_CHARS: [c_int; 10] = [0, 128, 256, 48, 54, -49, -1, i32::MIN, 127, 1];

/// `param3` values producing each reachable `selected_op` (`(param3 % 5) + 1`).
const P3_FOR_OP: [(c_int, c_int); 9] = [
    (-4, -3),
    (-3, -2),
    (-2, -1),
    (-1, 0),
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (4, 5),
];

/// `param4` values producing each reachable `second_op` (`((param4+1) % 5) + 1`).
const P4_FOR_OP: [(c_int, c_int); 9] = [
    (-5, -3),
    (-4, -2),
    (-3, -1),
    (-2, 0),
    (-1, 1),
    (0, 2),
    (1, 3),
    (2, 4),
    (3, 5),
];

#[test]
fn phase_b_mathop_all() {
    // ---- C32: fresh statics, history saturation, per-call transcripts -----
    // Runs first so that the `static` history really is empty.
    let mut expected_count = 0i64;
    for call in 0..12 {
        let t = diff_mathop(49 + call, 3 + call, call, call, &format!("C32 call={call}"));
        expected_count = (expected_count + 2).min(10);
        assert_eq!(
            field(&t, "History entries: "),
            expected_count,
            "C32 call={call}: static history must grow 2,4,6,8,10,10,…"
        );
    }

    // ---- C27: valid validation char × each in-range selected_op ------------
    for &p1 in &VALID_CHARS {
        for &(p3, op) in &P3_FOR_OP {
            if op < 1 {
                continue;
            }
            let t = diff_mathop(p1, 6, p3, 4, &format!("C27 p1={p1} op={op}"));
            assert_eq!(
                field(&t, "Operation priority: "),
                (op as i64) * 10,
                "C27: priority is selected_op * 10"
            );
        }
    }

    // ---- C28: invalid validation char (dead-store branch) ------------------
    for &p1 in &INVALID_CHARS {
        for &(p3, _) in &P3_FOR_OP {
            diff_mathop(p1, 7, p3, 5, &format!("C28 p1={p1} p3={p3}"));
        }
    }

    // ---- C29: every reachable selected_op, incl. 0 and negatives ----------
    for &(p3, op) in &P3_FOR_OP {
        let t = diff_mathop(50, 9, p3, 6, &format!("C29 p3={p3}"));
        assert_eq!(
            field(&t, "Operation priority: "),
            (op as i64) * 10,
            "C29 p3={p3}: negative/zero ops give negative/zero priority"
        );
    }
    // INT_MIN / INT_MAX derivations of selected_op.
    diff_mathop(50, 9, i32::MIN, 6, "C29 p3=INT_MIN");
    diff_mathop(50, 9, i32::MAX, 6, "C29 p3=INT_MAX");

    // ---- C30: every reachable second_op, incl. the param4 overflow --------
    for &(p4, _op) in &P4_FOR_OP {
        diff_mathop(51, 8, 2, p4, &format!("C30 p4={p4}"));
    }
    diff_mathop(51, 8, 2, i32::MAX, "C30 p4=INT_MAX (param4+1 overflows)");
    diff_mathop(51, 8, 2, i32::MIN, "C30 p4=INT_MIN");

    // ---- C31: div/mod guard reached inside mathop --------------------------
    // First computation: selected_op 4 or 5 with param2 == 0.
    for &(p3, op) in &P3_FOR_OP {
        if op == 4 || op == 5 {
            for &p4 in &[0i32, 1, -1, 7, i32::MAX, i32::MIN] {
                diff_mathop(52, 0, p3, p4, &format!("C31 op={op} param2=0 p4={p4}"));
            }
        }
    }
    // Second computation: `b` is `param4`, and `param4 == 0` forces
    // `second_op == 2` (multiply), so div/mod-by-zero is *unreachable* there.
    // Verified structurally, then the nearest reachable cases are exercised.
    for &(p4, op) in &P4_FOR_OP {
        assert!(
            !(p4 == 0 && (op == 4 || op == 5)),
            "second_op cannot be div/mod when param4 == 0"
        );
        if op == 4 || op == 5 {
            diff_mathop(53, 5, 1, p4, &format!("C31b second_op={op} p4={p4}"));
        }
    }

    // ---- C33: randomized quadruples ---------------------------------------
    let mut rng = Rng::new(0x33_0000_0001);
    let mut done = 0;
    let mut attempts = 0;
    while done < 400 && attempts < 4000 {
        attempts += 1;
        let p1 = rng.spicy_i32();
        let p2 = rng.spicy_i32();
        let p3 = rng.spicy_i32();
        let p4 = rng.spicy_i32();
        if mathop_is_ub(p1, p2, p3, p4) {
            continue;
        }
        diff_mathop(p1, p2, p3, p4, &format!("C33 #{done}"));
        done += 1;
    }
    assert_eq!(done, 400, "C33 should have produced 400 usable quadruples");

    // ---- C34: boundary quadruples (full cross product, UB pruned) ---------
    let vals = [i32::MIN, i32::MIN + 1, -128, -1, 0, 1, 127, 128, i32::MAX - 1, i32::MAX];
    let mut n = 0usize;
    let mut skipped = 0usize;
    for &p1 in &vals {
        for &p2 in &vals {
            for &p3 in &vals {
                for &p4 in &vals {
                    if mathop_is_ub(p1, p2, p3, p4) {
                        skipped += 1;
                        continue;
                    }
                    diff_mathop(p1, p2, p3, p4, "C34");
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n + skipped, vals.len().pow(4));
    assert!(n > 9000, "C34 exercised only {n} quadruples");
}
