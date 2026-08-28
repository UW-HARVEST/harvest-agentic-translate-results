//! Phase C row E1 — the NULL-pointer case.
//!
//! `c_src/src/lib.c` performs **no** NULL check: `path` goes straight into
//! `strrchr()`. Passing NULL is therefore undefined behaviour that faults, not
//! an error return, so it cannot be observed in-process without killing the
//! test harness.
//!
//! It is still a real, differentially-testable input: this file re-executes the
//! test binary as a child process, once against the C `.so` and once against
//! the Rust `.so`, and compares how each child *terminates*. Both children are
//! the same Rust test binary with the same signal disposition, so the
//! comparison is apples-to-apples.

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const TARGET_ENV: &str = "DRIVER_NULL_TARGET";
const WORKER: &str = "null_deref_worker";

/// Child-side worker. Ignored so it never runs during a normal `cargo test`;
/// the parent invokes it explicitly with `--ignored --exact`.
#[test]
#[ignore = "child process helper for null_pointer_differential"]
fn null_deref_worker() {
    let target = std::env::var(TARGET_ENV).unwrap_or_else(|_| {
        panic!("{TARGET_ENV} must be set to `c` or `rust` when running {WORKER}")
    });
    let driver = match target.as_str() {
        "c" => common::c_driver(),
        "rust" => common::rust_driver(),
        other => panic!("unknown {TARGET_ENV}={other}"),
    };

    // Flush the "about to fault" marker before the call so the parent can tell
    // a fault inside the call apart from a failure to even get here.
    eprintln!("worker: calling tool_basename(NULL) against {:?}", driver.which);

    let ret = unsafe { (driver.tool_basename)(std::ptr::null_mut()) };

    // Reached only if the implementation somehow tolerates NULL.
    println!("worker: survived, returned {ret:?}");
}

/// Outcome of one child run, in a form that can be compared exactly.
#[derive(Debug, PartialEq, Eq)]
enum Termination {
    Signal(i32),
    Exit(i32),
}

fn run_worker(target: &str) -> Termination {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--ignored", "--exact", WORKER, "--test-threads=1", "--nocapture"])
        .env(TARGET_ENV, target)
        // Children inherit the same .so locations as the parent.
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to spawn the worker child process");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("worker: calling tool_basename(NULL)"),
        "child for target `{target}` never reached the call site; \
         it failed during setup instead.\n--- stdout ---\n{}\n--- stderr ---\n{stderr}",
        String::from_utf8_lossy(&out.stdout)
    );

    match (out.status.signal(), out.status.code()) {
        (Some(sig), _) => Termination::Signal(sig),
        (None, Some(code)) => Termination::Exit(code),
        (None, None) => unreachable!("process neither signalled nor exited"),
    }
}

#[test]
fn null_pointer_differential() {
    let c = run_worker("c");
    let rust = run_worker("rust");

    assert_eq!(
        c, rust,
        "NULL input: the C and Rust implementations terminate differently \
         (C: {c:?}, Rust: {rust:?}). Both must fault identically, since the C \
         performs no NULL check."
    );

    // Pin down *which* behaviour they agree on, so a future change from
    // "both fault" to "both silently return" cannot pass unnoticed.
    match c {
        Termination::Signal(sig) => assert_eq!(
            sig, 11,
            "expected both to die on SIGSEGV (11) as `strrchr(NULL, ...)` does, got signal {sig}"
        ),
        Termination::Exit(code) => panic!(
            "neither implementation faulted on NULL (child exited with {code}); \
             the C dereferences NULL via strrchr, so this is a divergence from \
             the ground truth's observable behaviour"
        ),
    }
}
