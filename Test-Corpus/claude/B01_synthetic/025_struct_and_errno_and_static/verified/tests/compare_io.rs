// Integration test: run both the C binary and the Rust binary with the same
// stdin and verify their stdout is byte-identical.
//
// The project is a binary (not a library), so we exercise the full program
// end-to-end rather than calling individual functions through libloading.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary() -> PathBuf {
    // c_src/build/driver, relative to the package root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("c_src")
        .join("build")
        .join("driver")
}

fn rust_binary() -> PathBuf {
    // The Rust binary is built by cargo into target/<profile>/driver.
    // We use CARGO_BIN_EXE_<name>, which is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_stdin(path: &PathBuf, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn process");

    {
        let stdin = child.stdin.as_mut().expect("no stdin");
        stdin.write_all(input).expect("failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("failed to wait for process");
    output.stdout
}

fn assert_match(input: &[u8]) {
    let c_out = run_with_stdin(&c_binary(), input);
    let rust_out = run_with_stdin(&rust_binary(), input);
    assert_eq!(
        c_out,
        rust_out,
        "output mismatch for input {:?}\n--- C ---\n{}\n--- Rust ---\n{}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

#[test]
fn parses_zero() {
    assert_match(b"0\n");
}

#[test]
fn parses_positive_small() {
    assert_match(b"3\n");
}

#[test]
fn parses_positive_larger() {
    assert_match(b"42\n");
}

#[test]
fn parses_negative() {
    assert_match(b"-7\n");
}

#[test]
fn parses_explicit_plus() {
    assert_match(b"+10\n");
}

#[test]
fn parses_with_leading_whitespace() {
    assert_match(b"   12\n");
}

#[test]
fn parses_with_leading_tab() {
    assert_match(b"\t5\n");
}

#[test]
fn parses_int_max() {
    assert_match(b"2147483647\n");
}

#[test]
fn parses_int_min() {
    assert_match(b"-2147483648\n");
}

#[test]
fn out_of_range_overflow_high() {
    // 2147483648 is INT_MAX + 1 -> fails range check in C
    assert_match(b"2147483648\n");
}

#[test]
fn out_of_range_overflow_low() {
    // -2147483649 is INT_MIN - 1 -> fails range check in C
    assert_match(b"-2147483649\n");
}

#[test]
fn massively_overflowing_long() {
    // A value that overflows even long -> ERANGE in C
    assert_match(b"9999999999999999999999999999999999\n");
}

#[test]
fn empty_input() {
    // fgets returns NULL on empty input, but the buffer initialized to ""
    // will leave 'in' as the empty string. Then strtol("") -> endp == str,
    // so parse_val returns false.
    assert_match(b"");
}

#[test]
fn just_a_newline() {
    // strtol("\n") consumes whitespace and finds no digits -> endp == str
    assert_match(b"\n");
}

#[test]
fn non_numeric() {
    assert_match(b"hello\n");
}

#[test]
fn numeric_with_trailing_text() {
    // strtol stops at the non-digit, but parse_val only checks endp != str.
    // So "42abc" is accepted and x = 42.
    assert_match(b"42abc\n");
}

#[test]
fn whitespace_then_non_numeric() {
    assert_match(b"   xyz\n");
}

#[test]
fn just_a_minus() {
    // strtol("-") -> no digits -> endp == str -> parse_val returns false.
    assert_match(b"-\n");
}

#[test]
fn just_a_plus() {
    assert_match(b"+\n");
}

#[test]
fn input_no_trailing_newline() {
    // fgets reads up to EOF if no newline encountered
    assert_match(b"5");
}

#[test]
fn input_longer_than_buffer() {
    // fgets reads only sizeof(in)-1 = 99 chars
    let mut input = Vec::new();
    for _ in 0..120 {
        input.push(b'1');
    }
    input.push(b'\n');
    assert_match(&input);
}

#[test]
fn negative_extra_bedrooms() {
    assert_match(b"-2\n");
}

#[test]
fn large_extra_bedrooms() {
    assert_match(b"1000000\n");
}

#[test]
fn leading_zeros() {
    assert_match(b"00042\n");
}

#[test]
fn signed_zero_negative() {
    assert_match(b"-0\n");
}

#[test]
fn signed_zero_positive() {
    assert_match(b"+0\n");
}
