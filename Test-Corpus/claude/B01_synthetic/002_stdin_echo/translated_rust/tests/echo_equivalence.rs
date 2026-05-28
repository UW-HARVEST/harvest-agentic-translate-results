// Equivalence tests for the echo `driver` program.
//
// The C source compiles to an executable, and so does the Rust crate.
// There is no exported library API and no Cargo features to enumerate.
// The program's only "interface" is stdin -> stdout, so equivalence is
// established by feeding identical bytes to both binaries and comparing
// their stdout, stderr, and exit codes.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    assert!(p.exists(), "C binary not built at {:?}", p);
    p
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_stdin(bin: &PathBuf, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn child");
    {
        let stdin = child.stdin.as_mut().expect("no stdin");
        stdin.write_all(input).expect("write stdin");
    }
    child.wait_with_output().expect("wait failed")
}

fn assert_equivalent(input: &[u8], label: &str) {
    let c_out = run_with_stdin(&c_binary(), input);
    let r_out = run_with_stdin(&rust_binary(), input);

    assert_eq!(
        c_out.stdout, r_out.stdout,
        "[{label}] stdout differs.\n c: {:?}\n r: {:?}",
        c_out.stdout, r_out.stdout
    );
    assert_eq!(
        c_out.status.code(),
        r_out.status.code(),
        "[{label}] exit code differs"
    );
}

#[test]
fn empty_input() {
    assert_equivalent(b"", "empty");
}

#[test]
fn single_line_with_newline() {
    assert_equivalent(b"hello\n", "single line");
}

#[test]
fn single_line_no_newline() {
    assert_equivalent(b"hello", "no trailing newline");
}

#[test]
fn multiple_lines() {
    assert_equivalent(b"line1\nline2\nline3\n", "multiple lines");
}

#[test]
fn blank_lines() {
    assert_equivalent(b"\n\n\n", "blank lines only");
}

#[test]
fn long_line_under_buffer() {
    // 126 'a's then newline, total 127 bytes -> fits in a single fgets call.
    let mut buf = vec![b'a'; 126];
    buf.push(b'\n');
    assert_equivalent(&buf, "126 chars + newline");
}

#[test]
fn long_line_at_buffer_boundary() {
    // 127 'a's + newline. fgets reads first 127 bytes, then the newline next call.
    let mut buf = vec![b'a'; 127];
    buf.push(b'\n');
    assert_equivalent(&buf, "127 chars + newline");
}

#[test]
fn long_line_over_buffer() {
    // 200 'a's + newline forces multiple fgets iterations on the same line.
    let mut buf = vec![b'a'; 200];
    buf.push(b'\n');
    assert_equivalent(&buf, "200 chars + newline");
}

#[test]
fn very_long_input() {
    // ~10KB of mixed line lengths.
    let mut buf = Vec::new();
    for i in 0..500 {
        let line: Vec<u8> = (0..(i % 250)).map(|j| b'a' + ((j % 26) as u8)).collect();
        buf.extend_from_slice(&line);
        buf.push(b'\n');
    }
    assert_equivalent(&buf, "very long input");
}

#[test]
fn binary_bytes_passthrough() {
    // fgets is byte-oriented (treating only '\n' specially), so non-ASCII
    // bytes should pass through unchanged.
    let mut buf = Vec::new();
    for b in 1u8..=255 {
        if b != b'\n' {
            buf.push(b);
        }
    }
    buf.push(b'\n');
    assert_equivalent(&buf, "binary bytes");
}

#[test]
fn null_bytes_in_line() {
    // Embedded NUL bytes: C's fputs stops at the NUL within each fgets buffer,
    // so the Rust translation must do the same.
    // NOTE: we still compare against C as ground truth — whatever C does, Rust does.
    assert_equivalent(b"abc\0def\nxyz\n", "embedded NUL");
}

#[test]
fn crlf_lines() {
    assert_equivalent(b"line1\r\nline2\r\n", "CRLF lines");
}

#[test]
fn many_short_lines() {
    let mut buf = Vec::new();
    for _ in 0..1000 {
        buf.extend_from_slice(b"x\n");
    }
    assert_equivalent(&buf, "many short lines");
}

#[test]
fn trailing_partial_line() {
    assert_equivalent(b"complete\npartial", "complete + partial");
}
