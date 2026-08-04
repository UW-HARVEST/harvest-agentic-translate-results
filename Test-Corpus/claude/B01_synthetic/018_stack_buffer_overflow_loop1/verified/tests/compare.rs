// Integration tests comparing the C reference binary and the Rust translation.
//
// The C program is built as an executable (see c_src/CMakeLists.txt). It has no
// public ABI / shared library, so we compare end-to-end behavior by piping the
// same stdin into both binaries and asserting their stdout matches byte for byte.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary() -> PathBuf {
    // CARGO_MANIFEST_DIR points at translated_rust/.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    assert!(
        p.exists(),
        "C driver not built at {:?}. Run cmake build first.",
        p
    );
    p
}

fn rust_binary() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, input: &[u8]) -> (Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn binary");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input).expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait");
    (output.stdout, output.status.code())
}

fn assert_match(input: &[u8]) {
    let (c_out, c_code) = run(&c_binary(), input);
    let (r_out, r_code) = run(&rust_binary(), input);
    assert_eq!(
        c_out, r_out,
        "stdout mismatch for input {:?}: C={:?} Rust={:?}",
        input, c_out, r_out
    );
    assert_eq!(
        c_code, r_code,
        "exit code mismatch for input {:?}: C={:?} Rust={:?}",
        input, c_code, r_code
    );
}

#[test]
fn input_zero_takes_bad_branch() {
    assert_match(b"0");
}

#[test]
fn input_one_takes_good_branch() {
    assert_match(b"1");
}

#[test]
fn input_positive_takes_good_branch() {
    assert_match(b"42");
}

#[test]
fn input_negative_takes_good_branch() {
    assert_match(b"-5");
}

#[test]
fn input_empty_defaults_to_zero() {
    assert_match(b"");
}

#[test]
fn input_non_numeric_defaults_to_zero() {
    assert_match(b"abc");
}

#[test]
fn input_with_leading_whitespace() {
    assert_match(b"   3");
}

#[test]
fn input_with_leading_newline() {
    assert_match(b"\n7");
}

#[test]
fn input_with_trailing_garbage() {
    assert_match(b"5xyz");
}

#[test]
fn input_with_plus_sign() {
    assert_match(b"+9");
}

#[test]
fn input_with_minus_sign_zero() {
    assert_match(b"-0");
}

#[test]
fn input_with_only_whitespace() {
    assert_match(b"   \n\t  ");
}

#[test]
fn input_large_number() {
    assert_match(b"2147483647");
}

#[test]
fn input_large_negative() {
    assert_match(b"-2147483648");
}
