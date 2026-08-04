// Integration tests comparing the C driver and Rust driver binaries.
//
// The project builds both as executables (the C side via CMake; the Rust
// side via cargo). Both read a single float from stdin (scanf("%f")) and
// print the raw IEEE-754 bytes as hex followed by a newline. To verify
// byte-identical behavior we feed both binaries the same stdin and compare
// their stdout exactly.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> points to the binary built for tests.
    // Falls back to target/release/driver if not provided (e.g. when the
    // file integration test is run outside Cargo's harness).
    if let Some(path) = option_env!("CARGO_BIN_EXE_driver") {
        return PathBuf::from(path);
    }
    workspace_root().join("target/release/driver")
}

fn run_with_stdin(bin: &PathBuf, stdin_data: &[u8]) -> (Vec<u8>, i32) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(stdin_data).unwrap();
    }
    let out = child.wait_with_output().expect("wait_with_output");
    (out.stdout, out.status.code().unwrap_or(-1))
}

fn assert_match(input: &str) {
    let (c_out, c_rc) = run_with_stdin(&c_binary(), input.as_bytes());
    let (r_out, r_rc) = run_with_stdin(&rust_binary(), input.as_bytes());
    assert_eq!(
        c_out, r_out,
        "stdout mismatch for input {:?}\n  C: {:?}\n  R: {:?}",
        input,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(c_rc, r_rc, "exit code mismatch for input {:?}", input);
}

#[test]
fn matches_zero() {
    assert_match("0\n");
}

#[test]
fn matches_positive_one() {
    assert_match("1\n");
}

#[test]
fn matches_negative_one() {
    assert_match("-1\n");
}

#[test]
fn matches_one_point_five() {
    assert_match("1.5\n");
}

#[test]
fn matches_negative_zero() {
    assert_match("-0\n");
}

#[test]
fn matches_small_positive() {
    assert_match("3.14159\n");
}

#[test]
fn matches_small_negative() {
    assert_match("-2.71828\n");
}

#[test]
fn matches_scientific_notation() {
    assert_match("1.5e10\n");
}

#[test]
fn matches_negative_scientific() {
    assert_match("-1.5e-10\n");
}

#[test]
fn matches_large_value() {
    assert_match("3.4028235e38\n");
}

#[test]
fn matches_small_value() {
    assert_match("1.17549435e-38\n");
}

#[test]
fn matches_leading_whitespace() {
    assert_match("   42.0\n");
}

#[test]
fn matches_only_integer() {
    assert_match("100\n");
}

#[test]
fn matches_trailing_whitespace() {
    assert_match("7.5   \n");
}

#[test]
fn matches_no_input() {
    // scanf returns 0 conversions; C's x stays 0.f. Rust replicates that
    // by leaving x = 0.0.
    assert_match("");
}

#[test]
fn matches_just_whitespace() {
    assert_match("   \n");
}

#[test]
fn matches_explicit_plus_sign() {
    assert_match("+1.0\n");
}

#[test]
fn matches_decimal_no_int() {
    assert_match(".5\n");
}

#[test]
fn matches_int_no_decimal() {
    assert_match("5.\n");
}
