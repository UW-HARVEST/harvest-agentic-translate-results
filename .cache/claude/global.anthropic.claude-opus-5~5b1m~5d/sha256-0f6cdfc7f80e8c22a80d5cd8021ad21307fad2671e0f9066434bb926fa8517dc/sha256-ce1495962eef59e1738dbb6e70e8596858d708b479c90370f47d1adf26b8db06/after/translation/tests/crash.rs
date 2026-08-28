//! Phase C, part 1 — rejection paths that KILL the process.
//!
//! ERRORS.md rows #3 (`INT_MIN / -1`), #18 (`find_and_replace_char(NULL, ..)`) and
//! #19 (`process_octal_string(NULL, ..)`) cannot be checked in-process: the C traps.
//! So each case is run twice in a *child process* — once against the C `.so`, once
//! against the Rust `.so` — and the two terminations are compared: same fatal signal,
//! same exit code, same stdout. "Both died somehow" is not accepted; the signal
//! numbers must be equal.

mod common;
use common::*;

use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const WORKER_ENV_LIB: &str = "DIFFTEST_LIB";
const WORKER_ENV_CASE: &str = "DIFFTEST_CASE";

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: String,
}

fn run_case(which: &str, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "crash_worker", "--ignored", "--nocapture"])
        .env(WORKER_ENV_LIB, which)
        .env(WORKER_ENV_CASE, case)
        .output()
        .expect("spawn worker");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("RESULT:"))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[track_caller]
fn assert_same_termination(case: &str) {
    let c = run_case("c", case);
    let r = run_case("rust", case);
    assert_eq!(
        c, r,
        "\ncase `{case}` terminated differently:\n  C   : {c:?}\n  Rust: {r:?}"
    );
    eprintln!("case `{case}`: both terminated identically -> {c:?}");
}

// ---------------------------------------------------------------------------
// The child-side worker. Ignored so it never runs during a normal `cargo test`.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "spawned as a subprocess by the crash-parity tests"]
fn crash_worker() {
    let which = std::env::var(WORKER_ENV_LIB).expect("DIFFTEST_LIB");
    let case = std::env::var(WORKER_ENV_CASE).expect("DIFFTEST_CASE");
    let p = fresh_pair();
    let api = match which.as_str() {
        "c" => &p.c,
        "rust" => &p.r,
        other => panic!("bad DIFFTEST_LIB {other}"),
    };

    unsafe {
        match case.as_str() {
            // ERRORS.md #19 — process_octal_string(NULL, 0123): strcpy to NULL
            "octal_null" => {
                (api.process_octal_string)(std::ptr::null_mut(), 0o123);
                println!("RESULT: survived");
            }
            // ERRORS.md #18 — find_and_replace_char(NULL, 'O'): strlen(NULL)
            "replace_null" => {
                (api.find_and_replace_char)(std::ptr::null_mut(), b'O' as c_int);
                println!("RESULT: survived");
            }
            // ERRORS.md #3 — drive `multiplier` to INT_MIN, then divide by -1.
            "div_intmin_by_neg1" => {
                let m = (api.multiply_with_multiplier)(i32::MIN, 1);
                println!("RESULT: multiplier={m}");
                let d = (api.divide_multiplier)(0, -1);
                println!("RESULT: quotient={d}");
            }
            // Same overflow, reached with the operand order swapped.
            "div_intmin_by_neg1_swapped" => {
                let m = (api.multiply_with_multiplier)(-1, i32::MIN);
                println!("RESULT: multiplier={m}");
                let d = (api.divide_multiplier)(12345, -1);
                println!("RESULT: quotient={d}");
            }
            // Control: b == 0 must NOT trap (guarded by `if (b != 0)`).
            "div_by_zero_guarded" => {
                let m = (api.multiply_with_multiplier)(i32::MIN, 1);
                println!("RESULT: multiplier={m}");
                let d = (api.divide_multiplier)(0, 0);
                println!("RESULT: quotient={d}");
            }
            // Control: INT_MIN / -2 is representable, must not trap.
            "div_intmin_by_neg2" => {
                let m = (api.multiply_with_multiplier)(i32::MIN, 1);
                println!("RESULT: multiplier={m}");
                let d = (api.divide_multiplier)(0, -2);
                println!("RESULT: quotient={d}");
            }
            other => panic!("bad DIFFTEST_CASE {other}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Parent-side parity assertions
// ---------------------------------------------------------------------------

/// ERRORS.md #19
#[test]
fn err19_process_octal_string_null_dest() {
    assert_same_termination("octal_null");
}

/// ERRORS.md #18
#[test]
fn err18_find_and_replace_char_null_str() {
    assert_same_termination("replace_null");
}

/// ERRORS.md #3 — `multiplier == INT_MIN`, `b == -1`.
#[test]
fn err3_divide_multiplier_intmin_by_neg_one() {
    assert_same_termination("div_intmin_by_neg1");
}

/// ERRORS.md #3, second route to the same state.
#[test]
fn err3_divide_multiplier_intmin_by_neg_one_swapped() {
    assert_same_termination("div_intmin_by_neg1_swapped");
}

/// ERRORS.md #1 control — the `b != 0` guard means this must survive on both sides.
#[test]
fn err1_divide_by_zero_is_guarded_not_fatal() {
    let c = run_case("c", "div_by_zero_guarded");
    let r = run_case("rust", "div_by_zero_guarded");
    assert_eq!(c, r, "\nC: {c:?}\nRust: {r:?}");
    assert_eq!(c.signal, None, "b == 0 must not raise a signal: {c:?}");
    assert!(
        c.stdout.contains("quotient=-2147483648"),
        "guard should leave multiplier untouched: {c:?}"
    );
}

/// Control — `INT_MIN / -2` is representable, so no trap on either side.
#[test]
fn divide_intmin_by_neg_two_is_not_fatal() {
    let c = run_case("c", "div_intmin_by_neg2");
    let r = run_case("rust", "div_intmin_by_neg2");
    assert_eq!(c, r, "\nC: {c:?}\nRust: {r:?}");
    assert_eq!(c.signal, None, "INT_MIN / -2 must not trap: {c:?}");
}
