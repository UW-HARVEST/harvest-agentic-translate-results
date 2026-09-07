//! Phase C — the two rejections whose C ground truth KILLS THE PROCESS.
//! ERRORS.md rows 20 and 21.
//!
//! These cannot be compared in-process, so each case is run in a fresh child
//! process (the test binary re-executes itself) and the two libraries are
//! compared on their TERMINATION SIGNAL, which is the observable behaviour.

mod common;
use common::*;
use std::os::raw::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const SPEC_VAR: &str = "MATHOP_TRAP_SPEC";

/// How a child terminated.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_child(kind: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "zz_trap_child", "--nocapture", "--test-threads=1"])
        .env(SPEC_VAR, format!("{kind}|{which}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("spawn trap child");
    match out.status.signal() {
        Some(sig) => Outcome::Signal(sig),
        None => Outcome::Exit(out.status.code().unwrap_or(-1)),
    }
}

/// Assert the C and Rust `.so`s die (or survive) in exactly the same way.
fn assert_same(kind: &str, expect: Outcome) {
    let c = run_child(kind, "c");
    let r = run_child(kind, "rs");
    assert_eq!(
        c, r,
        "`{kind}`: C terminated as {c:?} but Rust terminated as {r:?} \
         — the two libraries must reject this input identically"
    );
    assert_eq!(c, expect, "`{kind}`: C did not behave as ERRORS.md predicts");
}

// ===========================================================================
// Row 20 — INT_MIN / -1 and INT_MIN % -1.
//
// Signed-division overflow is UB in C; on x86-64 `idiv` raises #DE, which the
// kernel delivers as SIGFPE (8). The Rust must die with the SAME signal — a
// silent `wrapping_div` (returning INT_MIN) or a Rust panic/abort (SIGABRT, 6)
// would both be divergences.
// ===========================================================================

const SIGFPE: i32 = 8;
const SIGSEGV: i32 = 11;

#[test]
fn err_20_divide_int_min_by_minus_one_traps() {
    assert_same("div_min_by_neg1", Outcome::Signal(SIGFPE));
}

#[test]
fn err_20_modulo_int_min_by_minus_one_traps() {
    assert_same("mod_min_by_neg1", Outcome::Signal(SIGFPE));
}

/// The same trap reached through `select_operation`'s returned pointer.
#[test]
fn err_20_trap_via_select_operation_pointer() {
    assert_same("select_div_trap", Outcome::Signal(SIGFPE));
    assert_same("select_mod_trap", Outcome::Signal(SIGFPE));
}

/// The same trap reached through the recording wrapper.
#[test]
fn err_20_trap_via_perform_computation_with_history() {
    assert_same("pcwh_div_trap", Outcome::Signal(SIGFPE));
    assert_same("pcwh_mod_trap", Outcome::Signal(SIGFPE));
}

/// The same trap reached through the ONLY entry point declared in `lib.h`.
/// `param3 = 3` selects OP_DIVIDE (3 % 5 + 1 == 4); `param3 = 4` selects
/// OP_MODULO. `param1 = INT_MIN`, `param2 = -1`.
#[test]
fn err_20_trap_via_public_mathop() {
    assert_same("mathop_div_trap", Outcome::Signal(SIGFPE));
    assert_same("mathop_mod_trap", Outcome::Signal(SIGFPE));
}

/// The second operation can trap too: `((param4 + 1) % 5) + 1 == 4` with
/// `param4 == -1` requires the intermediate result to be INT_MIN... but
/// `param4 == -1` gives op2 == 1. The reachable case is param4 == 2 (op2 == 4)
/// with intermediate == INT_MIN and param4 == -1, which cannot both hold, so
/// this asserts the NON-trapping outcome: a clean exit.
#[test]
fn err_20_second_op_cannot_trap_and_exits_cleanly() {
    // op2 == 4 (divide) needs param4 == 2; divisor is param4 == 2, never -1.
    assert_same("mathop_second_op_divide", Outcome::Exit(0));
}

/// Sanity control: a benign input must NOT kill either library. Without this,
/// the tests above could pass for the wrong reason (e.g. dlopen failing).
#[test]
fn err_20_control_benign_input_survives() {
    assert_same("benign", Outcome::Exit(0));
}

// ===========================================================================
// Row 21 — NULL out-parameters. `perform_computation_with_history` never
// null-checks `history` / `history_count`; it dereferences both
// unconditionally. Both libraries must segfault identically.
// ===========================================================================

#[test]
fn err_21_null_history_pointer_segfaults() {
    assert_same("null_history_ptr", Outcome::Signal(SIGSEGV));
}

#[test]
fn err_21_null_count_pointer_with_null_history_segfaults() {
    assert_same("null_count_ptr_null_history", Outcome::Signal(SIGSEGV));
}

#[test]
fn err_21_null_count_pointer_with_live_history_segfaults() {
    assert_same("null_count_ptr_live_history", Outcome::Signal(SIGSEGV));
}

#[test]
fn err_21_both_out_params_null_segfaults() {
    assert_same("null_both", Outcome::Signal(SIGSEGV));
}

/// A NULL history buffer with a valid, in-range count: the `*history == NULL`
/// branch replaces it, so this must NOT crash — the guard is on the buffer, not
/// on the pointer to it.
#[test]
fn err_21_null_buffer_is_recovered_not_an_error() {
    assert_same("null_buffer_recovered", Outcome::Exit(0));
}

// ===========================================================================
// The child. Does nothing unless MATHOP_TRAP_SPEC is set, so a plain
// `cargo test` run of this binary is unaffected.
// ===========================================================================
#[test]
fn zz_trap_child() {
    let spec = match std::env::var(SPEC_VAR) {
        Ok(s) => s,
        Err(_) => return, // parent-side invocation: no-op
    };
    let (kind, which) = spec.split_once('|').expect("spec is kind|which");
    let p = pair();
    let lib = if which == "c" { &p.c } else { &p.rs };

    let mut history: *mut ComputationResult = std::ptr::null_mut();
    let mut count: c_int = 0;

    match kind {
        "div_min_by_neg1" => {
            let v = lib.divide_operation(i32::MIN, -1, 0);
            println!("survived divide: {v}");
        }
        "mod_min_by_neg1" => {
            let v = lib.modulo_operation(i32::MIN, -1, 0);
            println!("survived modulo: {v}");
        }
        "select_div_trap" => {
            let f = lib.select_operation(4);
            let v = unsafe { f(i32::MIN, -1, 0) };
            println!("survived select divide: {v}");
        }
        "select_mod_trap" => {
            let f = lib.select_operation(5);
            let v = unsafe { f(i32::MIN, -1, 0) };
            println!("survived select modulo: {v}");
        }
        "pcwh_div_trap" => {
            let v = unsafe {
                lib.perform_computation_with_history(i32::MIN, -1, 4, &mut history, &mut count)
            };
            println!("survived pcwh divide: {v}");
        }
        "pcwh_mod_trap" => {
            let v = unsafe {
                lib.perform_computation_with_history(i32::MIN, -1, 5, &mut history, &mut count)
            };
            println!("survived pcwh modulo: {v}");
        }
        "mathop_div_trap" => {
            // param3 = 3 -> op1 = 3 % 5 + 1 = 4 = OP_DIVIDE
            let v = lib.mathop(i32::MIN, -1, 3, 1);
            println!("survived mathop divide: {v}");
        }
        "mathop_mod_trap" => {
            // param3 = 4 -> op1 = 4 % 5 + 1 = 5 = OP_MODULO
            let v = lib.mathop(i32::MIN, -1, 4, 1);
            println!("survived mathop modulo: {v}");
        }
        "mathop_second_op_divide" => {
            // param4 = 2 -> op2 = (2 + 1) % 5 + 1 = 4 = OP_DIVIDE, divisor 2.
            let v = lib.mathop(100, 7, 0, 2);
            println!("mathop second-op divide: {v}");
        }
        "benign" => {
            let v = lib.mathop(7, 3, 1, 1);
            let d = lib.divide_operation(-7, 2, 0);
            let m = lib.modulo_operation(-7, 2, 0);
            println!("benign: {v} {d} {m}");
        }
        "null_history_ptr" => {
            let v = unsafe {
                lib.perform_computation_with_history(1, 2, 1, std::ptr::null_mut(), &mut count)
            };
            println!("survived null history ptr: {v}");
        }
        "null_count_ptr_null_history" => {
            let v = unsafe {
                lib.perform_computation_with_history(
                    1,
                    2,
                    1,
                    &mut history,
                    std::ptr::null_mut(),
                )
            };
            println!("survived null count ptr: {v}");
        }
        "null_count_ptr_live_history" => {
            history = lib.allocate_results(10);
            assert!(!history.is_null());
            let v = unsafe {
                lib.perform_computation_with_history(
                    1,
                    2,
                    1,
                    &mut history,
                    std::ptr::null_mut(),
                )
            };
            println!("survived null count ptr (live history): {v}");
        }
        "null_both" => {
            let v = unsafe {
                lib.perform_computation_with_history(
                    1,
                    2,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            println!("survived both null: {v}");
        }
        "null_buffer_recovered" => {
            let v = unsafe {
                lib.perform_computation_with_history(1, 2, 1, &mut history, &mut count)
            };
            assert_eq!(v, 3);
            assert_eq!(count, 1);
            assert!(!history.is_null());
            println!("recovered null buffer: {v}");
        }
        other => panic!("unknown trap kind `{other}`"),
    }
}
