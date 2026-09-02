//! Phase C, row E3 — the unchecked null dereference.
//!
//! For `size >= -1` the C stores at least one element with no null check, so
//! `gaussian_kernel(NULL, size, radius)` is an unchecked dereference. The Rust
//! must be *equally* unchecked: a defensive early return would make Rust
//! survive where C dies, which is a behavioural divergence.
//!
//! The call is made in a forked child so the fault does not take the harness
//! down, and the two implementations are compared on their exact termination
//! status (same signal, or same exit code).

mod common;

use std::process::{Command, Stdio};

const VICTIM_ENV: &str = "HARVEST_NULL_VICTIM";
const SIZE_ENV: &str = "HARVEST_NULL_SIZE";

/// Child mode: dereference NULL through one of the two shared objects.
fn maybe_run_as_victim() {
    let which = match std::env::var(VICTIM_ENV) {
        Ok(v) => v,
        Err(_) => return,
    };
    let size: i32 = std::env::var(SIZE_ENV).unwrap().parse().unwrap();
    let p = common::pair();
    let f = match which.as_str() {
        "c" => p.c.gaussian_kernel,
        "rust" => p.rs.gaussian_kernel,
        other => panic!("bad victim {other}"),
    };
    unsafe {
        f(std::ptr::null_mut(), size, 3.0f32);
    }
    // If we get here the call survived the null pointer.
    std::process::exit(7);
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_victim(which: &str, size: i32) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .arg("--exact")
        .arg("e03_null_dereference_matches_c")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(VICTIM_ENV, which)
        .env(SIZE_ENV, size.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("spawn victim");

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = out.status.signal() {
            return Outcome::Signal(sig);
        }
    }
    Outcome::Exit(out.status.code().unwrap_or(-1))
}

#[test]
fn e03_null_dereference_matches_c() {
    // When this env var is set we ARE the child.
    maybe_run_as_victim();

    // Sizes that make the C perform at least one store => must fault in both.
    for size in [-1i32, 0, 1, 2, 3, 8, 33] {
        let c = run_victim("c", size);
        let r = run_victim("rust", size);
        assert_eq!(
            c, r,
            "size={size}: C and Rust must fail identically on a NULL dest.\n  \
             C    = {c:?}\n  Rust = {r:?}\n  \
             (a differing outcome means one side has a null check the other lacks)"
        );
        assert_eq!(
            c,
            Outcome::Signal(11),
            "size={size}: expected an unchecked dereference (SIGSEGV), got {c:?}. \
             If the platform reports something else, both sides still had to agree."
        );
    }

    // Sizes that make the C perform NO store => must survive in both (E1/E2).
    for size in [-2i32, -3, -1000, i32::MIN] {
        let c = run_victim("c", size);
        let r = run_victim("rust", size);
        assert_eq!(c, r, "size={size}: C and Rust must both survive a NULL dest");
        assert_eq!(
            c,
            Outcome::Exit(7),
            "size={size}: no store happens, so a NULL dest must be harmless"
        );
    }
}
