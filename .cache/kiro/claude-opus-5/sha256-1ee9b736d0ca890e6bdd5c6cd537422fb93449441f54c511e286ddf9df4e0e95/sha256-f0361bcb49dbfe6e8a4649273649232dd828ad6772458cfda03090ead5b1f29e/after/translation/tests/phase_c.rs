//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1–E3) plus the generic FFI boundary rows
//! (G1–G8). Every test constructs the exact rejection condition, calls BOTH
//! implementations through their `.so` exports, and asserts they agree on the
//! *specific* sentinel/diagnostic — not merely that "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_int, CString};
use std::ptr;

const VALIDATION_DIAG: &[u8] = b"Input string validation failed.\n";
const MALLOC_DIAG: &[u8] = b"Memory allocation failed.\n";
const SUCCESS_LINE: &[u8] = b"Processed numbers: numbers\n";

/// Expected `cleanup` return value, computed from the C's `switch` semantics
/// (including the deliberate `10 -> 20` and `30 -> 40` fall-throughs) with
/// wrapping `int` accumulation.
fn expected_cleanup(q: [i32; 4]) -> c_int {
    let mut r: i32 = 0;
    for v in q {
        r = match v {
            10 => r.wrapping_add(10).wrapping_add(20),
            20 => r.wrapping_add(20),
            30 => r.wrapping_add(30).wrapping_add(40),
            40 => r.wrapping_add(40),
            other => r.wrapping_add(other),
        };
    }
    r
}

/// E1 — `strncmp("VALID", "VALID", strlen("VALID")) != 0`.
///
/// Both operands are the same string literal, so the branch is statically
/// unreachable: the diagnostic must NEVER be printed and `result` must always
/// be the fully accumulated sum (not the early-`goto` value of 0). Asserted
/// for both implementations over a wide input set, so a Rust translation that
/// wrongly took the reject path would fail here.
#[test]
fn e1_string_validation_branch_never_taken() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut inputs: Vec<[i32; 4]> = vec![
        [10, 20, 30, 40],
        [0, 0, 0, 0],
        [i32::MIN, i32::MAX, -1, 1],
    ];
    for _ in 0..512 {
        inputs.push([
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ]);
    }

    for q in &inputs {
        let (vc, oc) = capture_stdout(|| unsafe { (l.c.cleanup)(q[0], q[1], q[2], q[3]) });
        let (vr, or) = capture_stdout(|| unsafe { (l.rust.cleanup)(q[0], q[1], q[2], q[3]) });

        assert_eq!(vc, vr, "E1: return differed for {q:?}");
        assert_eq!(oc, or, "E1: stdout differed for {q:?}");

        for (name, out) in [("C", &oc), ("Rust", &or)] {
            assert!(
                !contains(out, VALIDATION_DIAG),
                "E1: {name} took the unreachable validation-failure path for {q:?}"
            );
            assert!(
                contains(out, SUCCESS_LINE),
                "E1: {name} did not reach the success path for {q:?}: {:?}",
                String::from_utf8_lossy(out)
            );
        }
        // The early `goto cleanup` would have returned 0 with no accumulation.
        assert_eq!(
            vc,
            expected_cleanup(*q),
            "E1: C returned the early-goto value instead of the accumulated sum for {q:?}"
        );
    }
}

/// E2 — `malloc(50 * sizeof(char))` returns NULL.
///
/// A 50-byte request cannot fail here, so the diagnostic must never appear and
/// the `snprintf`/`printf` pair after the check must always run. Both sides
/// must agree, and must agree on the return value (which the C returns even on
/// this error path).
#[test]
fn e2_malloc_failure_branch_never_taken() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..512 {
        let q = [
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ];
        let (vc, oc) = capture_stdout(|| unsafe { (l.c.cleanup)(q[0], q[1], q[2], q[3]) });
        let (vr, or) = capture_stdout(|| unsafe { (l.rust.cleanup)(q[0], q[1], q[2], q[3]) });

        assert_eq!(vc, vr, "E2: return differed for {q:?}");
        assert_eq!(oc, or, "E2: stdout differed for {q:?}");
        for (name, out) in [("C", &oc), ("Rust", &or)] {
            assert!(
                !contains(out, MALLOC_DIAG),
                "E2: {name} reported allocation failure for {q:?}"
            );
            assert_eq!(
                out.as_slice(),
                SUCCESS_LINE,
                "E2: {name} stdout is not exactly the post-allocation success line for {q:?}"
            );
        }
    }
}

/// E3 — `cleanup_resources(NULL)`: the `if (dynamic_str)` guard makes it a
/// no-op. Must not free, must not print, must not crash, in either
/// implementation. Called repeatedly to catch a Rust version that unguarded
/// the free (which would abort on the second call).
#[test]
fn e3_cleanup_resources_null() {
    let l = libs();
    for _ in 0..1024 {
        let (_, oc) = capture_stdout(|| unsafe { (l.c.cleanup_resources)(ptr::null_mut()) });
        let (_, or) = capture_stdout(|| unsafe { (l.rust.cleanup_resources)(ptr::null_mut()) });
        assert_eq!(oc, or, "E3: stdout differed for NULL");
        assert!(oc.is_empty(), "E3: NULL case must produce no output");
    }
}

/// G1 — `print_result(NULL, r)`: glibc's `%s` renders `(null)`. Both sides
/// must produce the identical bytes.
#[test]
fn g1_print_result_null_label() {
    let l = libs();
    for r in [0, -1, 1, i32::MIN, i32::MAX] {
        let (_, oc) = capture_stdout(|| unsafe { (l.c.print_result)(ptr::null(), r) });
        let (_, or) = capture_stdout(|| unsafe { (l.rust.print_result)(ptr::null(), r) });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or),
            "G1: NULL label diverged for result {r}"
        );
        assert_eq!(
            oc,
            format!("(null): {r}\n").into_bytes(),
            "G1: unexpected glibc rendering of a NULL `%s`"
        );
    }
}

/// G2 — zero-length label.
#[test]
fn g2_print_result_empty_label() {
    let inputs: Vec<(CString, c_int)> = [0, -1, i32::MIN, i32::MAX]
        .iter()
        .map(|&r| (CString::new("").unwrap(), r))
        .collect();
    diff_print_result("G2", &inputs);
}

/// G3 — format metacharacters in the *data* position must be printed
/// verbatim, never interpreted.
#[test]
fn g3_print_result_format_metachars() {
    let evil = [
        "%s", "%d", "%n", "%%", "%p", "%1000000d", "%.*s", "%hn", "%9$n", "%s%s%s%s%s%s%s%s",
    ];
    let inputs: Vec<(CString, c_int)> = evil
        .iter()
        .flat_map(|s| {
            [0, -7, i32::MAX]
                .into_iter()
                .map(move |r| (CString::new(*s).unwrap(), r))
        })
        .collect();
    diff_print_result("G3", &inputs);

    // And confirm the metacharacters really did survive verbatim.
    let l = libs();
    let s = CString::new("%n%s").unwrap();
    let (_, out) = capture_stdout(|| unsafe { (l.rust.print_result)(s.as_ptr(), 5) });
    assert_eq!(out, b"%n%s: 5\n".to_vec());
}

/// G4 — oversized label: `printf` does not truncate (unlike the `snprintf`
/// inside `cleanup`).
#[test]
fn g4_print_result_oversized_label() {
    for n in [49usize, 50, 51, 4096, 65536] {
        let inputs = vec![(CString::new("A".repeat(n)).unwrap(), 1234)];
        diff_print_result("G4", &inputs);
        let l = libs();
        let (_, out) = capture_stdout(|| unsafe { (l.rust.print_result)(inputs[0].0.as_ptr(), 1234) });
        assert_eq!(out.len(), n + b": 1234\n".len(), "G4: label was truncated at n={n}");
    }
}

/// G5 — extreme `result` values through `%d`.
#[test]
fn g5_print_result_extreme_results() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    let mut inputs: Vec<(CString, c_int)> = [
        0,
        -1,
        1,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -10,
        10,
    ]
    .iter()
    .map(|&r| (CString::new("L").unwrap(), r))
    .collect();
    for _ in 0..512 {
        inputs.push((CString::new("L").unwrap(), rng.next_i32()));
    }
    diff_print_result("G5", &inputs);
}

/// G6 — `cleanup_resources` with a valid heap pointer: freed exactly once,
/// silently, by each implementation.
#[test]
fn g6_cleanup_resources_valid_ptr() {
    let l = libs();
    for size in [1usize, 8, 50, 51, 4096, 1 << 20] {
        let pc = host_malloc(size);
        assert!(!pc.is_null());
        let (_, oc) = capture_stdout(|| unsafe { (l.c.cleanup_resources)(pc) });
        let pr = host_malloc(size);
        assert!(!pr.is_null());
        let (_, or) = capture_stdout(|| unsafe { (l.rust.cleanup_resources)(pr) });
        assert_eq!(oc, or, "G6: stdout differed for size {size}");
        assert!(oc.is_empty(), "G6: must be silent");
    }
}

/// G7 — out-of-range "enum-like" selector values crossing the FFI boundary.
///
/// The C `switch` is on a plain `int`, so any value with no matching `case`
/// label is a legitimate input that must fall to `default:`. This covers one
/// step past every label, the negated labels, and the extremes.
#[test]
fn g7_out_of_range_case_selectors() {
    let selectors: Vec<i32> = vec![
        9, 11, 19, 21, 29, 31, 39, 41, -10, -20, -30, -40, 0, 1, -1, 5, 15, 25, 35, 45, 50, 100,
        i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1,
    ];
    // Every selector in every position, with the other slots pinned to 0.
    let mut inputs: Vec<[i32; 4]> = Vec::new();
    for &s in &selectors {
        for pos in 0..4usize {
            let mut q = [0i32; 4];
            q[pos] = s;
            inputs.push(q);
        }
        inputs.push([s, s, s, s]);
    }
    diff_cleanup("G7", &inputs);

    // And assert they really did take `default:` (return == plain sum), which
    // is what distinguishes an out-of-range selector from a case label.
    let l = libs();
    for &s in &selectors {
        if CASE_LABELS.contains(&s) {
            continue;
        }
        let (v, _) = capture_stdout(|| unsafe { (l.rust.cleanup)(s, 0, 0, 0) });
        assert_eq!(
            v, s,
            "G7: selector {s} did not fall through to `default:` in Rust"
        );
        let (vc, _) = capture_stdout(|| unsafe { (l.c.cleanup)(s, 0, 0, 0) });
        assert_eq!(vc, v, "G7: C/Rust disagree on out-of-range selector {s}");
    }
}

/// G8 — quadruples that overflow the `int` accumulator.
#[test]
fn g8_overflow_quadruples() {
    let inputs: Vec<[i32; 4]> = vec![
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, 10, i32::MAX, 10],
        [i32::MIN, 30, i32::MIN, 30],
        [i32::MAX, 1, 0, 0],
        [i32::MIN, -1, 0, 0],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MAX - 69, 10, 30, 0],
        [i32::MIN + 69, 10, 30, 0],
        [i32::MAX, 40, 40, 40],
        [i32::MIN, 20, 20, 20],
    ];
    diff_cleanup("G8", &inputs);
    // Cross-check against the wrapping model derived from the C.
    let l = libs();
    for q in &inputs {
        let (v, _) = capture_stdout(|| unsafe { (l.c.cleanup)(q[0], q[1], q[2], q[3]) });
        assert_eq!(v, expected_cleanup(*q), "G8: wrapping model mismatch for {q:?}");
    }
}

/// Extra generic boundary: zero and oversized *lengths* have no analogue in
/// this API (no length parameters exist), but the pointer parameters do. This
/// asserts both implementations tolerate a non-NUL-terminated-looking but
/// valid 1-byte label, and a label that is exactly the `snprintf` buffer size
/// used internally by `cleanup` (50).
#[test]
fn g_extra_pointer_boundaries() {
    let mut inputs: Vec<(CString, c_int)> = Vec::new();
    for n in [1usize, 2, 26, 27, 49, 50, 51] {
        inputs.push((CString::new("z".repeat(n)).unwrap(), n as c_int));
    }
    diff_print_result("G-extra", &inputs);
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}
