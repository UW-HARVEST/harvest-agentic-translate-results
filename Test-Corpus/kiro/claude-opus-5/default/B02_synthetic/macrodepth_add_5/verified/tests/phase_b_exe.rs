//! Phase B rows 26-27 — the composed `main` pipeline, compared process-to-process.
//!
//! `mdmain.c` is the only place `atoi`, the `printf` line ordering, the wrapping
//! `summary` sum and the exit status are all observable together, and it is not
//! part of either `.so`. Both `driver` executables are run with an identical
//! `argv[0]` (via `arg0`) so stderr compares byte-for-byte.

mod common;

use common::{c_exe_path, rust_exe_path, Rng, OP_TAG, REPEAT, SEED};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

struct Run {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(exe: &Path, args: &[&str]) -> Run {
    let mut cmd = Command::new(exe);
    cmd.arg0("PROG").args(args);
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    Run {
        status: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn assert_same_run(what: &str, args: &[&str]) {
    let c = run(&c_exe_path(), args);
    let r = run(&rust_exe_path(), args);
    assert_eq!(
        c.status, r.status,
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: exit status C={:?} Rust={:?}",
        c.status, r.status
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: stdout differs"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{OP_TAG}/{REPEAT}] {what} {args:?}: stderr differs"
    );
}

// Row 26 --------------------------------------------------------------------

#[test]
fn cfg_26_main_random_operand_pairs() {
    let mut rng = Rng::new(SEED ^ 0x26);
    for i in 0..512 {
        let a = rng.next_operand().to_string();
        let b = rng.next_operand().to_string();
        assert_same_run(&format!("main random#{i}"), &[&a, &b]);
    }
}

#[test]
fn cfg_26b_main_boundary_operand_pairs() {
    for &(a, b) in common::BOUNDARY_PAIRS {
        let (a, b) = (a.to_string(), b.to_string());
        assert_same_run("main boundary", &[&a, &b]);
    }
}

// Row 27 --------------------------------------------------------------------

#[test]
fn cfg_27_main_argv_lexical_shapes() {
    let shapes: &[&[&str]] = &[
        &["7", "3"],
        &["  -12abc", "+9"],
        &["007", "-0"],
        &["", ""],
        &["+", "-"],
        &["abc", "def"],
        &["12x", "7"],
        &[" \t\n\x0b\x0c\r42", "1"],
        &["2147483647", "2"],
        &["-2147483648", "-1"],
        &["2147483648", "0"],
        &["-2147483649", "0"],
        &["4294967296", "0"],
        &["99999999999999999999", "3"],
        &["-99999999999999999999", "3"],
        &["9223372036854775807", "1"],
        &["9223372036854775808", "1"],
        &["-9223372036854775808", "1"],
        &["-9223372036854775809", "1"],
        &["0x10", "10"],
        &["1e3", "2"],
        &["--5", "1"],
        &["+-5", "1"],
        &["5", "3", "IGNORED", "ALSO_IGNORED"],
        &["0000000000000000000000000000005", "3"],
        &["-0000000000000000000000000000005", "3"],
    ];
    for s in shapes {
        assert_same_run("main argv shape", s);
    }
}

#[test]
fn cfg_27b_main_repeat_dependent_lines() {
    // Pins the four printf lines mdmain.c emits for a fixed input, so a build
    // that silently ignored REPEAT would fail here rather than pass vacuously.
    let c = run(&c_exe_path(), &["7", "3"]);
    let text = String::from_utf8_lossy(&c.stdout).into_owned();
    let expected_acc: i32 = match OP_TAG {
        "add" => (0..REPEAT).sum(),
        "sub" => -(0..REPEAT).sum::<i32>(),
        _ => (0..REPEAT).fold(1, |acc, i| acc * (i + 1)),
    };
    assert!(
        text.contains(&format!("acc={expected_acc} ")),
        "[{OP_TAG}/{REPEAT}] expected acc={expected_acc} in {text:?}"
    );
    assert!(
        text.contains(&format!("op={OP_TAG} ")),
        "[{OP_TAG}/{REPEAT}] expected op={OP_TAG} in {text:?}"
    );
    assert_eq!(text.lines().count(), 5, "expected 5 output lines: {text:?}");
    let r = run(&rust_exe_path(), &["7", "3"]);
    assert_eq!(text, String::from_utf8_lossy(&r.stdout));
}
