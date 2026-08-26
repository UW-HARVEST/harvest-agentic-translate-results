//! Phase C — the ERRORS.md rows whose "expected C result" is abnormal process
//! termination: E6 (`add_node` with `name == NULL`), E17 (`process_string` with
//! `str == NULL`) and E12 (a parent/child cycle recursing until the stack is
//! exhausted).
//!
//! The C source performs no null check and has no cycle guard, so these inputs
//! are genuinely fatal. Each case is therefore run in a forked child process
//! (this same test binary, re-executed with `DIFF_CRASH_CASE` set) so the parent
//! harness survives and can compare HOW the two implementations died: the same
//! termination signal, not merely "both failed somehow".

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const ENV_CASE: &str = "DIFF_CRASH_CASE";

/// How a child ended.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signalled(i32),
}

fn run_child(case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .env(ENV_CASE, case)
        .env("RUST_BACKTRACE", "0")
        .args([
            "--exact",
            "crash_child_entry",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    match status.code() {
        Some(c) => Outcome::Exited(c),
        None => Outcome::Signalled(status.signal().expect("signal")),
    }
}

/// Compare the C child and the Rust child for one trigger.
fn assert_same_death(row: &str, c_case: &str, r_case: &str) {
    let c = run_child(c_case);
    let r = run_child(r_case);
    assert!(
        matches!(c, Outcome::Signalled(_)),
        "{row}: the C implementation was expected to die from a signal, got {c:?}"
    );
    assert_eq!(
        c, r,
        "{row}: C died as {c:?} but Rust died as {r:?} - they must terminate the same way"
    );
}

// ---------------------------------------------------------------------------
// The child. Runs exactly one fatal case and never returns normally from it.
// With no DIFF_CRASH_CASE in the environment (the normal parent run) it is a
// no-op, so it is harmless as an ordinary test.
// ---------------------------------------------------------------------------

#[test]
fn crash_child_entry() {
    let case = match std::env::var(ENV_CASE) {
        Ok(c) => c,
        Err(_) => return,
    };
    let p = Pair::new("crash-child");
    unsafe {
        match case.as_str() {
            // E6 - add_node(name = NULL) -> strncpy(dst, NULL, 49)
            "e6_c" => {
                (p.c.add_node)(1, -1, std::ptr::null(), 1.0);
            }
            "e6_r" => {
                (p.r.add_node)(1, -1, std::ptr::null(), 1.0);
            }
            // E17 - process_string(NULL) -> `if (*str)`
            "e17_c" => {
                (p.c.process_string)(std::ptr::null_mut());
            }
            "e17_r" => {
                (p.r.process_string)(std::ptr::null_mut());
            }
            // E12 - self-parented node: calculate_subtree_sum recurses forever
            "e12_c" => {
                (p.c.add_node)(7, 7, b"cycle\0".as_ptr() as *const _, 1.0);
                (p.c.calculate_subtree_sum)(7);
            }
            "e12_r" => {
                (p.r.add_node)(7, 7, b"cycle\0".as_ptr() as *const _, 1.0);
                (p.r.calculate_subtree_sum)(7);
            }
            // E12 - two-node mutual cycle (1 -> 2 -> 1)
            "e12b_c" => {
                (p.c.add_node)(1, 2, b"a\0".as_ptr() as *const _, 1.0);
                (p.c.add_node)(2, 1, b"b\0".as_ptr() as *const _, 2.0);
                (p.c.calculate_subtree_sum)(1);
            }
            "e12b_r" => {
                (p.r.add_node)(1, 2, b"a\0".as_ptr() as *const _, 1.0);
                (p.r.add_node)(2, 1, b"b\0".as_ptr() as *const _, 2.0);
                (p.r.calculate_subtree_sum)(1);
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    // Reaching here means the call did NOT terminate the process, which itself
    // is a divergence the parent will notice (Exited(0) instead of Signalled).
    println!("case {case} returned without crashing");
}

// ---------------------------------------------------------------------------
// The parent-side rows.
// ---------------------------------------------------------------------------

/// E6 — `add_node` with `name == NULL`: `strncpy` dereferences it unchecked.
#[test]
fn e6_add_node_null_name_dies_identically() {
    assert_same_death("E6 add_node(name=NULL)", "e6_c", "e6_r");
}

/// E17 — `process_string(NULL)`: `if (*str)` dereferences it unchecked.
#[test]
fn e17_process_string_null_dies_identically() {
    assert_same_death("E17 process_string(NULL)", "e17_c", "e17_r");
}

/// E12 — a self-parented node makes `calculate_subtree_sum` recurse forever;
/// there is no cycle guard in the C source.
#[test]
fn e12_self_cycle_dies_identically() {
    assert_same_death("E12 self cycle", "e12_c", "e12_r");
}

/// E12 — the same for a two-node mutual cycle.
#[test]
fn e12b_mutual_cycle_dies_identically() {
    assert_same_death("E12 mutual cycle", "e12b_c", "e12b_r");
}

/// Sanity check on the child mechanism itself: with no case set the child exits
/// cleanly, so a "did not crash" result really is distinguishable from a crash.
#[test]
fn crash_harness_child_is_a_noop_without_the_env_var() {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .env_remove(ENV_CASE)
        .args(["--exact", "crash_child_entry", "--test-threads", "1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn");
    assert_eq!(status.code(), Some(0), "control child must exit 0");
}
