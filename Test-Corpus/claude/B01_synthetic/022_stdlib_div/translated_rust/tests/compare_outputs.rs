// Compare the C driver and Rust driver behavior across many input cases.
//
// The original C source builds a single executable (`driver`) that reads
// two integers from stdin via scanf("%d %d") and prints the quotient and
// remainder of div(x, y).
//
// Since this is an executable and not a shared library, this test spawns
// both binaries as subprocesses and compares their stdout / exit status
// byte-for-byte for a variety of stdin inputs.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary_path() -> PathBuf {
    // Allow override via env var, otherwise use the default cmake build path.
    if let Ok(p) = std::env::var("C_DRIVER_BIN") {
        return PathBuf::from(p);
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/driver")
}

fn rust_binary_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_BIN") {
        return PathBuf::from(p);
    }
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests when there is
    // a binary target named <name> in the package. Our binary is named
    // `driver`.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    success: bool,
    code: Option<i32>,
}

fn run_with_input(bin: &PathBuf, input: &[u8]) -> RunResult {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    {
        let stdin = child.stdin.as_mut().expect("failed to open stdin");
        // Write may fail if the child process has already exited or closed
        // stdin (e.g., on a divide-by-zero trap). Ignore such errors.
        let _ = stdin.write_all(input);
    }

    let output = child.wait_with_output().expect("failed to wait on child");
    RunResult {
        stdout: output.status.success().then(|| output.stdout.clone()).unwrap_or(output.stdout),
        stderr: output.stderr,
        success: output.status.success(),
        code: output.status.code(),
    }
}

/// Compare both binaries on the given stdin input. Asserts that:
///   - stdout bytes match exactly
///   - both succeed (or both fail) -- we don't compare exit codes exactly
///     because divide-by-zero produces a SIGFPE in C and a panic in Rust
///     and those exit indicators differ. We DO compare success/failure.
fn assert_equiv(input: &[u8]) {
    let c_path = c_binary_path();
    let rust_path = rust_binary_path();
    let c = run_with_input(&c_path, input);
    let r = run_with_input(&rust_path, input);

    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch for input {:?}\n  C stdout: {:?}\n  Rust stdout: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.success, r.success,
        "exit-status (success/failure) mismatch for input {:?}: C success={} (code={:?}), Rust success={} (code={:?})",
        String::from_utf8_lossy(input),
        c.success, c.code,
        r.success, r.code,
    );
}

#[test]
fn basic_positive_division() {
    assert_equiv(b"10 3\n");
}

#[test]
fn exact_division() {
    assert_equiv(b"12 4\n");
}

#[test]
fn negative_dividend() {
    assert_equiv(b"-10 3\n");
}

#[test]
fn negative_divisor() {
    assert_equiv(b"10 -3\n");
}

#[test]
fn both_negative() {
    assert_equiv(b"-10 -3\n");
}

#[test]
fn zero_dividend() {
    assert_equiv(b"0 5\n");
}

#[test]
fn one_one() {
    assert_equiv(b"1 1\n");
}

#[test]
fn large_values() {
    assert_equiv(b"2147483647 2\n");
}

#[test]
fn min_int_div_one() {
    // INT_MIN / 1 -- safe, result is INT_MIN.
    assert_equiv(b"-2147483648 1\n");
}

#[test]
fn extra_whitespace() {
    assert_equiv(b"   42    7   \n");
}

#[test]
fn tabs_and_newlines() {
    assert_equiv(b"\t\n100\n\t  25\n");
}

#[test]
fn explicit_plus_signs() {
    assert_equiv(b"+25 +4\n");
}

#[test]
fn no_newline_at_end() {
    assert_equiv(b"7 2");
}

#[test]
fn empty_input() {
    // No input: both default x=1, y=1, so should print "quotient: 1, remainder: 0".
    assert_equiv(b"");
}

#[test]
fn only_first_value() {
    // Only x is read; y stays at default 1.
    assert_equiv(b"42");
}

#[test]
fn invalid_input_letters() {
    // scanf("%d %d", &x, &y) fails to read either; defaults stay 1, 1.
    assert_equiv(b"abc def\n");
}

#[test]
fn first_valid_then_garbage() {
    // First int is read; second is not. y stays at 1.
    assert_equiv(b"5 abc\n");
}

#[test]
fn negative_int_min_plus_one() {
    assert_equiv(b"-2147483647 7\n");
}

#[test]
fn small_positive_remainder() {
    assert_equiv(b"7 3\n");
}

#[test]
fn one_divided_by_two() {
    assert_equiv(b"1 2\n");
}
