//! Phase C, continued — rows that terminate the process by design.
//!
//! `ERRORS.md` rows E2 (SIGFPE on `INT_MIN % -1`), E24 (negative-count runaway),
//! E25 (NULL `operation_func`) and E34/E35 (NULL pointers) cannot be checked
//! in-process, so each is run in a **child process** and the two
//! implementations' *termination status* (signal number, or exit code) is
//! compared. This is what makes "both reject it the same way" a real assertion
//! rather than "both failed somehow".
//!
//! The child is this very test binary, re-executed with `PROBE_CASE` /
//! `PROBE_LIB` set, running only the `probe_child` test.

mod common;

use common::*;
use std::os::raw::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::ptr;

const CASE_ENV: &str = "PROBE_CASE";
const LIB_ENV: &str = "PROBE_LIB";

/// How a child process ended.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Signal(s) => write!(f, "killed by signal {s} ({})", signame(*s)),
            Outcome::Exit(c) => write!(f, "exited with code {c}"),
        }
    }
}

fn signame(s: i32) -> &'static str {
    match s {
        4 => "SIGILL",
        6 => "SIGABRT",
        8 => "SIGFPE",
        11 => "SIGSEGV",
        _ => "?",
    }
}

/// Runs one probe case against one implementation in a fresh process.
fn run_probe(case: &str, which: &str) -> Outcome {
    let l = libs_release();
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["probe_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(CASE_ENV, case)
        .env(LIB_ENV, which)
        // Pass the resolved paths so the child never re-runs cargo.
        .env("C_SO_PATH", &l.c.path)
        .env("RUST_SO_PATH", &l.rust.path)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("spawn probe child");

    match out.status.signal() {
        Some(s) => Outcome::Signal(s),
        None => Outcome::Exit(out.status.code().unwrap_or(-1)),
    }
}

/// Asserts the C and Rust children terminate identically, and (when given) that
/// the shared outcome is the specific one the C source dictates.
#[track_caller]
fn assert_same_outcome(row: &str, case: &str, expected: Option<Outcome>) {
    let c = run_probe(case, "c");
    let r = run_probe(case, "rust");
    assert_eq!(
        c, r,
        "{row} [{case}]: implementations rejected the input differently\n  \
         C    {c}\n  Rust {r}"
    );
    if let Some(exp) = expected {
        assert_eq!(
            c, exp,
            "{row} [{case}]: both agreed on `{c}` but the C source dictates `{exp}`"
        );
    }
}

// ===========================================================================
// The child side
// ===========================================================================

/// Runs a single probe case in-process. Only ever reached in a child process
/// (i.e. when `PROBE_CASE` is set); in a normal test run it returns immediately.
#[test]
fn probe_child() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) if !c.is_empty() => c,
        _ => return, // normal run: nothing to do
    };
    let which = std::env::var(LIB_ENV).unwrap_or_default();
    let l = libs_release();
    let imp = match which.as_str() {
        "c" => &l.c,
        "rust" => &l.rust,
        other => panic!("bad {LIB_ENV}={other}"),
    };

    // A generously oversized backing buffer so that the *runaway* cases march
    // through memory the same way in both children rather than immediately
    // trampling this frame's return address.
    let mut backing: Vec<ResultArray> = vec![ResultArray::dirty(1); 4096];
    let arr: *mut ResultArray = backing.as_mut_ptr();

    unsafe {
        match case.as_str() {
            // ---- E2: INT_MIN % -1 raises #DE -> SIGFPE -------------------
            "e2_intmin_rem_minus_one" => {
                let v = (imp.modulo_operation)(i32::MIN, -1, 0, 0);
                println!("modulo_operation(INT_MIN,-1) = {v}");
            }
            "e2_intmin_rem_minus_one_via_process" => {
                // Same trap reached through the composed pipeline.
                (*arr).count = 1;
                (*arr).data[0].value = i32::MIN;
                (*arr).data[0].rank = -1;
                let v = (imp.process_with_foreach)(arr, Some(imp.modulo_operation));
                println!("process_with_foreach = {v}");
            }

            // ---- E24: negative count -> FOREACH never terminates ---------
            "e24_negative_count_process" => {
                (*arr).count = -1;
                let v = (imp.process_with_foreach)(arr, Some(imp.add_operation));
                println!("process_with_foreach(count=-1) = {v}");
            }

            // ---- E25: NULL op with count > 0 -----------------------------
            "e25_null_op_nonzero_count" => {
                (*arr).count = 8;
                let v = (imp.process_with_foreach)(arr, None);
                println!("process_with_foreach(op=NULL) = {v}");
            }

            // ---- E34: NULL ResultArray ----------------------------------
            "e34_null_arr_compare" => {
                let v = (imp.compare_results_in_array)(ptr::null_mut(), 0, 1);
                println!("compare_results_in_array(NULL) = {v}");
            }
            "e34_null_arr_init" => {
                let mut vals: [c_int; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
                (imp.init_result_array)(ptr::null_mut(), vals.as_mut_ptr(), 8);
                println!("init_result_array(NULL) returned");
            }
            "e34_null_arr_process" => {
                let v = (imp.process_with_foreach)(ptr::null_mut(), Some(imp.add_operation));
                println!("process_with_foreach(NULL) = {v}");
            }
            "e34_null_arr_weighted" => {
                let v = (imp.compute_weighted_sum)(ptr::null_mut());
                println!("compute_weighted_sum(NULL) = {v}");
            }

            // ---- E35: NULL values[] with count > 0 ----------------------
            "e35_null_values_init" => {
                (imp.init_result_array)(arr, ptr::null_mut(), 8);
                println!("init_result_array(values=NULL, 8) returned");
            }

            // ---- Controls: these must NOT crash -------------------------
            "control_null_op_zero_count" => {
                (*arr).count = 0;
                let v = (imp.process_with_foreach)(arr, None);
                assert_eq!(v, 0, "control: expected 0");
                println!("ok");
            }
            "control_null_values_zero_count" => {
                (imp.init_result_array)(arr, ptr::null_mut(), 0);
                println!("ok");
            }
            "control_null_values_negative_count" => {
                (imp.init_result_array)(arr, ptr::null_mut(), -5);
                println!("ok");
            }
            "control_modulo_by_zero" => {
                let v = (imp.modulo_operation)(i32::MIN, 0, 0, 0);
                assert_eq!(v, 0, "control: expected 0");
                println!("ok");
            }
            "control_arrayfunc" => {
                let v = (imp.arrayfunc)(1, 2, 3, 4);
                println!("arrayfunc(1,2,3,4) = {v}");
            }
            other => panic!("unknown probe case `{other}`"),
        }
    }

    // Keep the buffer alive across the FFI calls above.
    std::hint::black_box(&backing);
}

// ===========================================================================
// The parent side — one test per ERRORS.md row
// ===========================================================================

#[test]
fn e2_intmin_remainder_raises_sigfpe_in_both() {
    // The C evaluates `a % b` with a single `idivl`; for INT_MIN % -1 the
    // implicit quotient overflows and the CPU raises #DE => SIGFPE (signal 8).
    assert_same_outcome(
        "E2 modulo_operation(INT_MIN, -1)",
        "e2_intmin_rem_minus_one",
        Some(Outcome::Signal(8)),
    );
}

#[test]
fn e2b_intmin_remainder_through_pipeline_raises_sigfpe_in_both() {
    assert_same_outcome(
        "E2 process_with_foreach + modulo_operation(INT_MIN, -1)",
        "e2_intmin_rem_minus_one_via_process",
        Some(Outcome::Signal(8)),
    );
}

#[test]
fn e24_negative_count_kills_both_processes() {
    // `FOREACH` terminates on `count_iter != size`, which never holds for a
    // negative `size`, so the loop marches forward writing 24 bytes at a time
    // until the process dies.
    //
    // The *exact* signal is genuinely nondeterministic for BOTH implementations:
    // the runaway writes eventually either touch an unmapped page (SIGSEGV) or
    // corrupt glibc's heap metadata first, making a later allocator call
    // `abort()` (SIGABRT). Repeated runs of the unmodified libraries show the C
    // and the Rust each producing both signals, so requiring signal equality
    // here would be asserting a coin flip, not a translation property.
    //
    // What IS assertable — and asserted:
    //   * neither implementation returns normally (both are killed by a signal);
    //   * the identical out-of-bounds address arithmetic and per-element writes,
    //     verified deterministically by
    //     `phase_c_errors::e24_deterministic_out_of_bounds_marching`, which uses
    //     a large *mapped* backing buffer and a big positive `count` to exercise
    //     exactly the same marching code path byte-for-byte.
    let c = run_probe("e24_negative_count_process", "c");
    let r = run_probe("e24_negative_count_process", "rust");
    for (which, o) in [("C", &c), ("Rust", &r)] {
        match o {
            Outcome::Signal(s) => assert!(
                matches!(s, 11 | 6 | 7 | 4),
                "E24 {which}: unexpected signal {s} ({})",
                signame(*s)
            ),
            Outcome::Exit(code) => panic!(
                "E24 {which}: process returned normally (exit {code}); the \
                 negative-count loop must not terminate"
            ),
        }
    }
}

#[test]
fn e25_null_op_with_nonzero_count_faults_in_both() {
    assert_same_outcome(
        "E25 process_with_foreach(op = NULL, count = 8)",
        "e25_null_op_nonzero_count",
        None,
    );
}

#[test]
fn e34_null_result_array_faults_in_both() {
    for case in [
        "e34_null_arr_compare",
        "e34_null_arr_init",
        "e34_null_arr_process",
        "e34_null_arr_weighted",
    ] {
        assert_same_outcome("E34 NULL ResultArray*", case, Some(Outcome::Signal(11)));
    }
}

#[test]
fn e35_null_values_with_nonzero_count_faults_in_both() {
    assert_same_outcome(
        "E35 init_result_array(values = NULL, count = 8)",
        "e35_null_values_init",
        Some(Outcome::Signal(11)),
    );
}

#[test]
fn controls_do_not_crash_in_either_implementation() {
    // Negative controls: these prove the probe harness reports success when the
    // library is *supposed* to tolerate the input, so the assertions above are
    // not trivially passing.
    for case in [
        "control_null_op_zero_count",
        "control_null_values_zero_count",
        "control_null_values_negative_count",
        "control_modulo_by_zero",
        "control_arrayfunc",
    ] {
        assert_same_outcome("control (must succeed)", case, Some(Outcome::Exit(0)));
    }
}
