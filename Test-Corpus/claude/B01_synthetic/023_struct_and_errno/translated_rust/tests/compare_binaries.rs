// Integration test that runs both the C binary and the Rust binary with the
// same stdin input, and compares their stdout output byte-for-byte.
//
// Note: The project under test produces an executable (not a shared library).
// `c_src/CMakeLists.txt` builds `driver` as an executable, and the C code
// declares `add_floor`, `add_bedrooms`, `print_house`, and `parse_val` as
// `static` (file-scope) — only `main` and `run` have external linkage.
// Because `c_src/` may not be modified to produce a shared library, the
// appropriate equivalence check at this granularity is to drive both binaries
// through their public interface (stdin -> stdout) and compare.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary_path() -> PathBuf {
    // Built by `cmake --build` into c_src/build/driver
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    p
}

fn rust_binary_path() -> PathBuf {
    // CARGO_BIN_EXE_<name> points at the Rust binary built for tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_stdin(bin: &PathBuf, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input).expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(label: &str, input: &[u8]) {
    let c = c_binary_path();
    let r = rust_binary_path();

    assert!(
        c.exists(),
        "C binary not found at {:?}; build it first with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`",
        c
    );
    assert!(r.exists(), "Rust binary not found at {:?}", r);

    let (c_out, _c_err, c_code) = run_with_stdin(&c, input);
    let (r_out, _r_err, r_code) = run_with_stdin(&r, input);

    if c_out != r_out || c_code != r_code {
        panic!(
            "Mismatch for case '{}':\n  input bytes: {:?}\n  C   exit: {:?}\n  Rust exit: {:?}\n  C   stdout ({} bytes): {:?}\n  Rust stdout ({} bytes): {:?}\n",
            label,
            input,
            c_code,
            r_code,
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            r_out.len(),
            String::from_utf8_lossy(&r_out),
        );
    }
}

#[test]
fn case_zero() {
    assert_match("zero", b"0\n");
}

#[test]
fn case_positive_small() {
    assert_match("positive small", b"3\n");
}

#[test]
fn case_positive_large() {
    assert_match("positive large", b"123456\n");
}

#[test]
fn case_negative_small() {
    assert_match("negative small", b"-1\n");
}

#[test]
fn case_negative_large() {
    assert_match("negative large", b"-987654\n");
}

#[test]
fn case_int_max() {
    assert_match("int max", b"2147483647\n");
}

#[test]
fn case_int_min() {
    assert_match("int min", b"-2147483648\n");
}

#[test]
fn case_int_max_plus_one_overflow() {
    // 2^31 = 2147483648, which overflows int32. parse_val should fail.
    assert_match("int max +1 overflow", b"2147483648\n");
}

#[test]
fn case_int_min_minus_one_overflow() {
    assert_match("int min -1 overflow", b"-2147483649\n");
}

#[test]
fn case_huge_overflow_long() {
    // Definitely overflows even 64-bit long; strtol errno=ERANGE => fail.
    assert_match("huge overflow long", b"99999999999999999999\n");
}

#[test]
fn case_empty_input() {
    // fgets returns NULL/zero bytes; in[0] remains '\0'. parse_val gets ""
    // which fails ("endp == str").
    assert_match("empty input", b"");
}

#[test]
fn case_no_digits_just_newline() {
    assert_match("only newline", b"\n");
}

#[test]
fn case_leading_whitespace_then_digit() {
    // strtol skips leading whitespace.
    assert_match("leading whitespace", b"   42\n");
}

#[test]
fn case_plus_sign() {
    assert_match("plus sign", b"+7\n");
}

#[test]
fn case_only_sign_no_digits() {
    // strtol with just "-" leaves endp==str => failure.
    assert_match("only minus", b"-\n");
}

#[test]
fn case_garbage() {
    assert_match("garbage", b"abc\n");
}

#[test]
fn case_digits_then_garbage() {
    // strtol consumes leading digits and stops; the prefix counts as success.
    assert_match("digits then garbage", b"12abc\n");
}

#[test]
fn case_no_trailing_newline() {
    assert_match("no trailing newline", b"5");
}

#[test]
fn case_one() {
    assert_match("one", b"1\n");
}
