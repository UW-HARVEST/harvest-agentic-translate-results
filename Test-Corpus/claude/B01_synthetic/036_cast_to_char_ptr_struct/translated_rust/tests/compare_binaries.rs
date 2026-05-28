// Compares the C executable to the Rust executable for the same inputs.
// The C side is `c_src/build/driver` (built via cmake).
// The Rust side is the Cargo binary `driver`.
//
// This project is a binary-only translation (no shared library, no Cargo
// features), so we cannot use libloading to exercise individual exported
// functions — there are no exported symbols. We instead drive both
// executables on stdin and compare their stdout byte-for-byte.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    manifest_dir().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // env!("CARGO_BIN_EXE_<name>") gives us the path of the built bin.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_stdin(bin: &PathBuf, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    output.stdout
}

fn assert_match(input: &[u8]) {
    let c = run_with_stdin(&c_binary(), input);
    let r = run_with_stdin(&rust_binary(), input);
    assert_eq!(
        c, r,
        "Output mismatch for input {:?}\nC:    {}\nRust: {}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r),
    );
}

#[test]
fn match_zero() {
    assert_match(b"0\n");
}

#[test]
fn match_positive() {
    assert_match(b"1\n");
    assert_match(b"5\n");
    assert_match(b"42\n");
    assert_match(b"100\n");
    assert_match(b"12345\n");
}

#[test]
fn match_negative() {
    assert_match(b"-1\n");
    assert_match(b"-5\n");
    assert_match(b"-42\n");
}

#[test]
fn match_int_min_max() {
    assert_match(b"2147483647\n");
    assert_match(b"-2147483648\n");
}

#[test]
fn match_signed_with_plus() {
    assert_match(b"+5\n");
    assert_match(b"+0\n");
}

#[test]
fn match_leading_whitespace() {
    assert_match(b"   42\n");
    assert_match(b"\t\n  7\n");
    assert_match(b"\n\n\n\t  -7\n");
}

#[test]
fn match_octal_like_input() {
    // %d with C scanf treats "0042" as decimal 42.
    assert_match(b"0042\n");
}

#[test]
fn match_no_input() {
    // C scanf returns 0 matches; x stays at its initialized value 0.
    assert_match(b"");
}

#[test]
fn match_non_numeric_input() {
    // No digits parsed; x stays 0.
    assert_match(b"abc");
    assert_match(b"hello world");
}

#[test]
fn match_hex_like_prefix() {
    // %d stops at 'x' so "0x10" parses as 0.
    assert_match(b"0x10\n");
}

#[test]
fn match_trailing_garbage() {
    assert_match(b"123abc\n");
    assert_match(b"-9 hello\n");
}
