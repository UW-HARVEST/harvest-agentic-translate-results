// Integration test: compares the C binary and the Rust binary byte-for-byte.
//
// The C target is an executable (see c_src/CMakeLists.txt) and so is the Rust
// crate's `driver` binary. There is no public API exposed via a shared
// library, so libloading cannot be used here. Instead we run both binaries
// as subprocesses with identical stdin and assert their stdout matches
// exactly.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary_path() -> PathBuf {
    // c_src/build/driver, relative to the crate root (CARGO_MANIFEST_DIR).
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    p
}

fn rust_binary_path() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn child");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input).expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait child");
    (output.stdout, output.stderr, output.status.code())
}

fn assert_match(input: &[u8]) {
    let c = c_binary_path();
    let r = rust_binary_path();
    assert!(c.exists(), "C binary not built: {}", c.display());
    assert!(r.exists(), "Rust binary not built: {}", r.display());
    let (c_out, _c_err, c_code) = run(&c, input);
    let (r_out, _r_err, r_code) = run(&r, input);
    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for input {:?}\nC stdout: {:?}\nRust stdout: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    assert_eq!(c_code, r_code, "exit codes differ for input {:?}", input);
}

#[test]
fn basic_disjoint() {
    // strcspn("hello", "world") = 2 (the 'l' in hello is in "world")
    assert_match(b"hello\nworld\n");
}

#[test]
fn first_char_matches() {
    // strcspn("abc", "a") = 0
    assert_match(b"abc\na\n");
}

#[test]
fn no_chars_match() {
    // strcspn("abc", "xyz") = 3
    assert_match(b"abc\nxyz\n");
}

#[test]
fn empty_first_string() {
    // First line empty -> after stripping the newline becomes empty.
    // strcspn("", "abc") = 0
    assert_match(b"\nabc\n");
}

#[test]
fn empty_second_string() {
    // strcspn("abc", "") = 3
    assert_match(b"abc\n\n");
}

#[test]
fn long_strings() {
    // Just under the 100-char buffer for both lines.
    let mut input = Vec::new();
    input.extend(std::iter::repeat(b'a').take(50));
    input.push(b'\n');
    input.extend(std::iter::repeat(b'b').take(50));
    input.push(b'\n');
    assert_match(&input);
}

#[test]
fn match_in_middle() {
    // strcspn("abcdef", "d") = 3
    assert_match(b"abcdef\nd\n");
}

#[test]
fn whitespace_chars() {
    // mix of spaces and tabs
    assert_match(b"a b c\n \n");
}

#[test]
fn all_match() {
    // strcspn("aaaa", "a") = 0
    assert_match(b"aaaa\na\n");
}

#[test]
fn punctuation() {
    assert_match(b"hello, world!\n,\n");
}
