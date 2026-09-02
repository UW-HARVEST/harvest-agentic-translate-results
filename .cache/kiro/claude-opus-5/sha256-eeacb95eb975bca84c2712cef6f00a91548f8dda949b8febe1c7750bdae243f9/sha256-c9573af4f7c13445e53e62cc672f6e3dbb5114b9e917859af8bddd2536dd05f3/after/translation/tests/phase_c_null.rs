//! Phase C, row 12 — the null-pointer path.
//!
//! `flac_validate` has no null check in the C: its first statement is
//! `t->blocksize`. Passing NULL is therefore undefined behaviour that traps.
//! Both implementations must trap the SAME way, so the probe runs in a forked
//! child (re-exec of this very test binary) and the parent compares the fatal
//! signals. This keeps the harness alive while still asserting parity.

mod common;
use common::*;

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// Child-side probe. A no-op unless `NULL_PROBE` selects an implementation,
/// so it stays green during ordinary `cargo test` runs.
#[test]
fn null_probe_child() {
    let which = match std::env::var("NULL_PROBE") {
        Ok(v) => v,
        Err(_) => return,
    };
    let p = pair();
    let target = match which.as_str() {
        "c" => &p.c,
        "rust" => &p.rs,
        other => panic!("unknown NULL_PROBE={other}"),
    };
    eprintln!("[null_probe] calling {} flac_validate(NULL)", target.name);
    let null: *mut Tflac = std::ptr::null_mut();
    // SAFETY: none — intentionally reproducing the C's unchecked dereference.
    let out = unsafe { target.flac_validate_raw(null) };
    // Unreachable in practice; printed so an unexpected survival is visible.
    println!("SURVIVED rc={out}");
}

#[test]
fn e12_null_pointer_traps_identically() {
    let exe = std::env::current_exe().expect("current_exe");
    let run = |which: &str| {
        Command::new(&exe)
            .args(["--exact", "null_probe_child", "--nocapture", "--test-threads=1"])
            .env("NULL_PROBE", which)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child probe")
    };

    let c = run("c");
    let r = run("rust");

    let cs = c.status.signal();
    let rs = r.status.signal();
    eprintln!(
        "[E12] C: signal={:?} code={:?} | Rust: signal={:?} code={:?}",
        cs,
        c.status.code(),
        rs,
        r.status.code()
    );

    assert!(
        cs.is_some(),
        "expected the C probe to die from a signal, got code={:?}\nstdout: {}\nstderr: {}",
        c.status.code(),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(
        cs, rs,
        "[E12] flac_validate(NULL) must trap identically: C signal {cs:?}, Rust signal {rs:?}"
    );
    assert_eq!(cs, Some(11), "[E12] expected SIGSEGV (11)");
    assert!(
        !String::from_utf8_lossy(&c.stdout).contains("SURVIVED"),
        "C probe unexpectedly survived a NULL dereference"
    );
    assert!(
        !String::from_utf8_lossy(&r.stdout).contains("SURVIVED"),
        "Rust probe unexpectedly survived a NULL dereference"
    );
}
