//! Phase C, fatal-signal rows — `ERRORS.md` rows 7 and 19.
//!
//! `process_pointer_data(NULL, ...)` and `apply_operation(NULL, ...)` are
//! unchecked in `lib.c`, so the C process dies. Asserting "both failed somehow"
//! would be too weak, so each case is run in a **child process** for the C `.so`
//! and again for the Rust `.so`, and the two are required to die with the
//! **same** termination signal.
//!
//! The children are `#[ignore]`d tests in this same binary, re-executed via
//! `current_exe --ignored --exact <name>`.

mod harness;

use harness::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const ENV_MARKER: &str = "HATCH_CRASH_CHILD";

fn is_child() -> bool {
    std::env::var_os(ENV_MARKER).is_some()
}

/// Runs `test_name` (an `#[ignore]`d test in this binary) in a child process
/// with `HATCH_CRASH_CHILD` set, and returns `(signal, exit_code)`.
fn run_child(test_name: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--ignored", "--exact", "--test-threads=1", test_name])
        .env(ENV_MARKER, "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawning the child test process");
    (status.signal(), status.code())
}

#[track_caller]
fn assert_same_fatal_behaviour(case: &str, c_test: &str, r_test: &str) {
    let (c_sig, c_code) = run_child(c_test);
    let (r_sig, r_code) = run_child(r_test);

    assert!(
        c_sig.is_some() || c_code != Some(0),
        "{case}: the C library was expected to fault, but the child exited cleanly"
    );
    assert_eq!(
        c_sig, r_sig,
        "{case}: DIVERGENCE in termination signal — C died with {c_sig:?} (exit {c_code:?}) \
         but Rust died with {r_sig:?} (exit {r_code:?})"
    );
    assert_eq!(
        c_code, r_code,
        "{case}: DIVERGENCE in exit code — C {c_code:?}, Rust {r_code:?}"
    );
}

// ===========================================================================
// Row 7 — process_pointer_data(NULL, multiplier): unchecked deref of address 0
// ===========================================================================

#[test]
#[ignore = "child process: intentionally faults"]
fn child_c_process_pointer_data_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    let v = unsafe { (p.c.process_pointer_data)(std::ptr::null_mut(), 3) };
    // Unreachable: the deref above must fault.
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
#[ignore = "child process: intentionally faults"]
fn child_rust_process_pointer_data_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    let v = unsafe { (p.r.process_pointer_data)(std::ptr::null_mut(), 3) };
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
fn err07_process_pointer_data_null_pointer_parity() {
    if is_child() {
        return;
    }
    assert_same_fatal_behaviour(
        "process_pointer_data(NULL, 3)",
        "child_c_process_pointer_data_null",
        "child_rust_process_pointer_data_null",
    );
}

// ===========================================================================
// Row 19 — apply_operation(NULL, ...): unchecked indirect call through address 0
// ===========================================================================

#[test]
#[ignore = "child process: intentionally faults"]
fn child_c_apply_operation_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    let v = unsafe { (p.c.apply_operation)(None, 1, 2, 3) };
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
#[ignore = "child process: intentionally faults"]
fn child_rust_apply_operation_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    let v = unsafe { (p.r.apply_operation)(None, 1, 2, 3) };
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
fn err19_apply_operation_null_callback_parity() {
    if is_child() {
        return;
    }
    assert_same_fatal_behaviour(
        "apply_operation(NULL, 1, 2, 3)",
        "child_c_apply_operation_null",
        "child_rust_apply_operation_null",
    );
}

// ===========================================================================
// Row 12/16 corner — manipulate_records(NULL, ...) with a POSITIVE loop bound.
// The lib.c:111 guard does not fire, but the lib.c:116 loop still runs, so the
// C dereferences NULL. Same fatal signal required.
// ===========================================================================

#[test]
#[ignore = "child process: intentionally faults"]
fn child_c_manipulate_records_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    // num_records = 4, shift = 0 -> guard false, loop bound 4 -> reads NULL[0].
    let v = unsafe { (p.c.manipulate_records)(std::ptr::null_mut(), 4, 0) };
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
#[ignore = "child process: intentionally faults"]
fn child_rust_manipulate_records_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    let v = unsafe { (p.r.manipulate_records)(std::ptr::null_mut(), 4, 0) };
    println!("unexpectedly survived: {v}");
    std::process::exit(0);
}

#[test]
fn manipulate_records_null_with_positive_bound_parity() {
    if is_child() {
        return;
    }
    assert_same_fatal_behaviour(
        "manipulate_records(NULL, 4, 0)",
        "child_c_manipulate_records_null",
        "child_rust_manipulate_records_null",
    );
}

// ===========================================================================
// shift_array_data(NULL, size, shift_by) with the guard SATISFIED: the C calls
// memmove/memset on NULL, so glibc faults. Same signal required.
// ===========================================================================

#[test]
#[ignore = "child process: intentionally faults"]
fn child_c_shift_array_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    unsafe { (p.c.shift_array_data)(std::ptr::null_mut(), 8, 3) };
    println!("unexpectedly survived");
    std::process::exit(0);
}

#[test]
#[ignore = "child process: intentionally faults"]
fn child_rust_shift_array_null() {
    assert!(is_child(), "only run as a child process");
    let p = libs();
    unsafe { (p.r.shift_array_data)(std::ptr::null_mut(), 8, 3) };
    println!("unexpectedly survived");
    std::process::exit(0);
}

#[test]
fn shift_array_data_null_with_guard_satisfied_parity() {
    if is_child() {
        return;
    }
    assert_same_fatal_behaviour(
        "shift_array_data(NULL, 8, 3)",
        "child_c_shift_array_null",
        "child_rust_shift_array_null",
    );
}

// ===========================================================================
// Generic-boundary null-pointer cases that do NOT fault, for completeness:
// shift_array_data / manipulate_records / compute_with_dynamic_memory reject
// their inputs before touching the pointer, so NULL is survivable there.
// ===========================================================================

#[test]
fn null_pointers_that_the_guards_make_survivable() {
    if is_child() {
        return;
    }
    let _g = lock();
    let p = libs();

    // shift_array_data(NULL, size, shift_by) is a no-op whenever the lib.c:67
    // guard rejects, so the NULL is never dereferenced. Both must survive and
    // agree (void return; the observable is "did not fault").
    for (size, shift_by) in [
        (0, 0),
        (0, 1),
        (0, -1),
        (-1, 1),
        (1, 1),
        (1, 0),
        (5, 5),
        (5, 9),
        (5, -3),
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (1, i32::MAX),
    ] {
        unsafe { (p.c.shift_array_data)(std::ptr::null_mut(), size, shift_by) };
        unsafe { (p.r.shift_array_data)(std::ptr::null_mut(), size, shift_by) };
    }

    // manipulate_records(NULL, n, shift) likewise, whenever the loop bound is
    // non-positive and the guard is false.
    for (n, shift) in [
        (0, 0),
        (0, 1),
        (0, i32::MAX),
        (-1, 0),
        (-1, 1),
        (5, 5),
        (5, 9),
        (i32::MAX, i32::MIN),
        (1, i32::MAX),
    ] {
        let cv = unsafe { (p.c.manipulate_records)(std::ptr::null_mut(), n, shift) };
        let rv = unsafe { (p.r.manipulate_records)(std::ptr::null_mut(), n, shift) };
        assert_eq_ctx(
            format!("manipulate_records(NULL, {n}, {shift})"),
            cv,
            rv,
        );
        assert_eq!(cv, 0, "manipulate_records(NULL, {n}, {shift}) should be 0");
    }

    // compute_with_dynamic_memory never takes a pointer; count <= 0 makes its
    // own (failed) allocation harmless.
    for count in [0, -1, i32::MIN] {
        let cv = unsafe { (p.c.compute_with_dynamic_memory)(12345, count) };
        let rv = unsafe { (p.r.compute_with_dynamic_memory)(12345, count) };
        assert_eq_ctx(format!("compute_with_dynamic_memory(12345, {count})"), cv, rv);
    }

    // A non-null but 1-element buffer with an oversized `size`/`num_records`
    // is still rejected by the guards when shift is out of range.
    let mut one: c_int = 0x1234_5678;
    let cv = unsafe { (p.c.process_pointer_data)(&mut one, 2) };
    let rv = unsafe { (p.r.process_pointer_data)(&mut one, 2) };
    assert_eq_ctx("process_pointer_data(&one, 2)", cv, rv);
}
