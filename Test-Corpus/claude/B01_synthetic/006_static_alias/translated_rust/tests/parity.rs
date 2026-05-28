// Parity tests: compare C executable output against Rust executable output
// for byte-identical stdout, stderr, and exit code across many inputs.
//
// The translated program is a small CLI binary (no shared library and no
// FFI exports), so we compare process output rather than invoking symbols
// through libloading.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set for integration tests by Cargo.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

#[derive(Debug, PartialEq, Eq)]
struct RunOutput {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(bin: &PathBuf, args: &[&str]) -> RunOutput {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", bin, e));
    RunOutput {
        status: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn assert_parity(args: &[&str]) {
    let c = run(&c_binary(), args);
    let r = run(&rust_binary(), args);
    assert_eq!(
        c.status, r.status,
        "exit code mismatch for args {:?}: C={:?} Rust={:?}",
        args, c.status, r.status
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch for args {:?}\nC:  {}\nRust: {}",
        args,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr mismatch for args {:?}\nC:  {}\nRust: {}",
        args,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
}

#[test]
fn no_args() {
    assert_parity(&[]);
}

#[test]
fn one_arg() {
    assert_parity(&["5"]);
}

#[test]
fn three_args() {
    assert_parity(&["5", "10", "extra"]);
}

#[test]
fn first_arg_not_integer() {
    assert_parity(&["foo", "10"]);
}

#[test]
fn second_arg_not_integer() {
    assert_parity(&["5", "bar"]);
}

#[test]
fn empty_first_arg() {
    assert_parity(&["", "5"]);
}

#[test]
fn empty_second_arg() {
    assert_parity(&["5", ""]);
}

#[test]
fn zero_iterations() {
    assert_parity(&["5", "0"]);
}

#[test]
fn negative_iterations() {
    assert_parity(&["5", "-3"]);
}

#[test]
fn small_positive() {
    assert_parity(&["5", "10"]);
}

#[test]
fn initial_zero() {
    assert_parity(&["0", "10"]);
}

#[test]
fn initial_one() {
    assert_parity(&["1", "10"]);
}

#[test]
fn initial_negative() {
    assert_parity(&["-3", "8"]);
}

#[test]
fn initial_negative_large_iter() {
    assert_parity(&["-100", "20"]);
}

#[test]
fn large_initial() {
    assert_parity(&["100", "15"]);
}

#[test]
fn boundary_cases() {
    assert_parity(&["1", "1"]);
    assert_parity(&["2", "2"]);
    assert_parity(&["10", "1"]);
}

#[test]
fn leading_whitespace() {
    // strtol skips leading whitespace
    assert_parity(&["   42", "5"]);
    assert_parity(&["5", "   3"]);
}

#[test]
fn signed_input() {
    assert_parity(&["+7", "5"]);
    assert_parity(&["-7", "5"]);
}

#[test]
fn trailing_garbage() {
    // strtol parses prefix and stops; some characters may be accepted.
    assert_parity(&["42abc", "5"]);
    assert_parity(&["5", "3xyz"]);
}

#[test]
fn just_sign() {
    // "+" or "-" alone parses no digits: end == nptr.
    assert_parity(&["+", "5"]);
    assert_parity(&["-", "5"]);
    assert_parity(&["5", "+"]);
    assert_parity(&["5", "-"]);
}

#[test]
fn matrix_small() {
    for init in [-5_i32, -1, 0, 1, 2, 3, 7, 10] {
        for iters in [0_i32, 1, 2, 5, 10] {
            let a = init.to_string();
            let b = iters.to_string();
            assert_parity(&[a.as_str(), b.as_str()]);
        }
    }
}
