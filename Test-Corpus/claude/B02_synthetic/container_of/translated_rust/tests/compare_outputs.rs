// Integration tests: compare output of the C binary and the Rust binary
// for the container_of demo program. The C source builds an executable
// (per c_src/CMakeLists.txt), and the Rust crate also builds a binary
// of the same name. There are no Cargo features and no shared library
// is built, so we compare process outputs directly.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // The Rust binary is compiled by Cargo before running tests; CARGO_BIN_EXE_<name>
    // is set when the crate has a [[bin]] target with that name.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, args: &[&str]) -> (i32, Vec<u8>, Vec<u8>) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", bin, e));
    (out.status.code().unwrap_or(-1), out.stdout, out.stderr)
}

fn compare(args: &[&str]) {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary not built at {:?} – run cmake --build first", c);
    let (cs, co, ce) = run(&c, args);
    let (rs, ro, re) = run(&r, args);
    assert_eq!(cs, rs, "exit status mismatch for args {:?}\nC stderr={}\nRust stderr={}",
        args, String::from_utf8_lossy(&ce), String::from_utf8_lossy(&re));
    assert_eq!(
        co, ro,
        "stdout mismatch for args {:?}\nC: {:?}\nRust: {:?}",
        args,
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro)
    );
}

#[test]
fn small_positive_pair() {
    compare(&["1", "2"]);
}

#[test]
fn zeros() {
    compare(&["0", "0"]);
}

#[test]
fn negatives() {
    compare(&["-3", "-7"]);
}

#[test]
fn mixed_signs() {
    compare(&["-10", "20"]);
}

#[test]
fn larger_values() {
    compare(&["12345", "67890"]);
}

#[test]
fn whitespace_prefix() {
    // atoi(3) skips leading whitespace
    compare(&["   42", "\t-5"]);
}

#[test]
fn trailing_garbage() {
    // atoi stops at first non-digit
    compare(&["42abc", "13xyz"]);
}

#[test]
fn plus_sign() {
    compare(&["+11", "+22"]);
}

#[test]
fn empty_strings_invalid() {
    // atoi("") returns 0
    compare(&["", ""]);
}
