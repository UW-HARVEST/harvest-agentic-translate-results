use std::process::{Command, Stdio};
use std::io::Write;
use std::path::PathBuf;

fn c_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    // Build the rust binary first
    let status = Command::new("cargo")
        .args(["build", "--bin", "driver"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to build rust binary");
    assert!(status.success(), "rust binary build failed");

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/driver")
}

fn run_binary_with_input(binary: &PathBuf, input: &str) -> Vec<u8> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", binary, e));

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).unwrap();
    }

    let output = child.wait_with_output().expect("failed to wait");
    output.stdout
}

#[test]
fn test_main_valid_input_3() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "3\n");
    let r_out = run_binary_with_input(&rust_bin, "3\n");

    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "stdout mismatch for input '3'"
    );
    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input '3'");
}

#[test]
fn test_main_valid_input_0() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "0\n");
    let r_out = run_binary_with_input(&rust_bin, "0\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input '0'");
}

#[test]
fn test_main_valid_input_negative() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "-5\n");
    let r_out = run_binary_with_input(&rust_bin, "-5\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input '-5'");
}

#[test]
fn test_main_invalid_input() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "abc\n");
    let r_out = run_binary_with_input(&rust_bin, "abc\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for invalid input 'abc'");
}

#[test]
fn test_main_empty_input() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "\n");
    let r_out = run_binary_with_input(&rust_bin, "\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for empty input");
}

#[test]
fn test_main_large_value() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "100\n");
    let r_out = run_binary_with_input(&rust_bin, "100\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input '100'");
}

#[test]
fn test_main_overflow_value() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    // Value exceeding INT_MAX
    let c_out = run_binary_with_input(&c_bin, "99999999999999\n");
    let r_out = run_binary_with_input(&rust_bin, "99999999999999\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for overflow input");
}

#[test]
fn test_main_leading_whitespace() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    let c_out = run_binary_with_input(&c_bin, "  7\n");
    let r_out = run_binary_with_input(&rust_bin, "  7\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input with leading whitespace");
}

#[test]
fn test_main_trailing_text() {
    let c_bin = c_binary();
    let rust_bin = rust_binary();

    // C's strtol parses "42abc" as 42 with endp pointing to 'a'
    let c_out = run_binary_with_input(&c_bin, "42abc\n");
    let r_out = run_binary_with_input(&rust_bin, "42abc\n");

    assert_eq!(c_out, r_out, "byte-for-byte mismatch for input '42abc'");
}
