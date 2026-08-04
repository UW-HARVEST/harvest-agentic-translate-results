// Integration tests: run both C and Rust drivers as subprocesses, feeding the
// same stdin to each, and assert their stdouts match byte-for-byte.
//
// The C and Rust programs are stand-alone executables (no shared library and
// no exported FFI symbols — `foo` is `static` in C). We treat the binaries as
// the public interface and compare end-to-end behavior.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests when there is
    // a [[bin]] target with that name.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_input(bin: &PathBuf, input: &str) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn driver");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input.as_bytes()).expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait child");
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(input: &str) {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary missing: {}", c.display());
    assert!(r.exists(), "Rust binary missing: {}", r.display());

    let (c_out, _c_err, c_code) = run_with_input(&c, input);
    let (r_out, _r_err, r_code) = run_with_input(&r, input);

    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for input {:?}\nC:\n{}\nRust:\n{}",
        input,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    assert_eq!(c_code, r_code, "exit code mismatch for input {:?}", input);
}

#[test]
fn zero_zero() {
    assert_match("0 0\n");
}

#[test]
fn one_zero() {
    assert_match("1 0\n");
}

#[test]
fn zero_one() {
    assert_match("0 1\n");
}

#[test]
fn one_one() {
    assert_match("1 1\n");
}

#[test]
fn two_three() {
    assert_match("2 3\n");
}

#[test]
fn three_two() {
    assert_match("3 2\n");
}

#[test]
fn one_four_special_case() {
    // Triggers the `if (x == 1 && y == 4) goto label2;` branch.
    assert_match("1 4\n");
}

#[test]
fn five_seven() {
    assert_match("5 7\n");
}

#[test]
fn seven_five() {
    assert_match("7 5\n");
}

#[test]
fn ten_ten() {
    assert_match("10 10\n");
}

#[test]
fn boundary_three_three() {
    // x < 3 transitions occur here.
    assert_match("3 3\n");
}

#[test]
fn boundary_four_four() {
    assert_match("4 4\n");
}

#[test]
fn x_greater_than_y() {
    assert_match("8 2\n");
}

#[test]
fn y_greater_than_x() {
    assert_match("2 8\n");
}

#[test]
fn x_zero_y_positive() {
    assert_match("0 5\n");
}

#[test]
fn x_positive_y_zero() {
    assert_match("5 0\n");
}

#[test]
fn whitespace_separated() {
    // scanf %d %d skips arbitrary whitespace.
    assert_match("  3\t4  \n");
}

#[test]
fn newline_separated() {
    assert_match("2\n3\n");
}

#[test]
fn no_input_uses_zeros() {
    // With no readable digits, both treat x=y=0 and produce no output.
    assert_match("");
}

#[test]
fn negative_numbers() {
    // x and y are negative -> while condition false immediately.
    assert_match("-1 -2\n");
}

#[test]
fn large_values() {
    assert_match("15 20\n");
}

#[test]
fn one_three() {
    // Edge near the special case (x==1, y==4).
    assert_match("1 3\n");
}

#[test]
fn one_five() {
    // Near the special case but should not trigger it on first iteration.
    assert_match("1 5\n");
}

#[test]
fn two_four() {
    assert_match("2 4\n");
}
