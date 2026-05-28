// Integration test: ensure the Rust translation produces byte-identical
// output to the original C implementation for the `driver` program.
//
// Both the C source and the Rust source build as executables that read a
// single integer from stdin (via `scanf("%d", ...)` / equivalent in Rust)
// and print `2*x + 300` followed by a newline (via `printf("%d\n", ...)`).
//
// Because both sides are programs with `main()` (the C side cannot be built
// as a shared library without modifying c_src/, which is forbidden), the
// only meaningful "FFI boundary" between them is the process boundary:
// stdin in, stdout out. We exercise that boundary directly by spawning both
// binaries with the same input and comparing their stdout byte-for-byte.

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
    // Cargo sets CARGO_BIN_EXE_<name> for the package's bins when building
    // integration tests, so we get the freshly-built driver binary path.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_input(path: &PathBuf, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", path, e));

    {
        let stdin = child.stdin.as_mut().expect("no stdin handle");
        // Best-effort write — the child may close stdin early on parse errors.
        let _ = stdin.write_all(input);
    }
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {:?}: {}", path, e));
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(input: &[u8]) {
    let c_path = c_binary();
    let r_path = rust_binary();
    assert!(
        c_path.exists(),
        "C binary not found at {:?}; run cmake build first",
        c_path
    );
    assert!(
        r_path.exists(),
        "Rust binary not found at {:?}; cargo should build it",
        r_path
    );

    let (c_out, _c_err, c_code) = run_with_input(&c_path, input);
    let (r_out, _r_err, r_code) = run_with_input(&r_path, input);

    assert_eq!(
        c_out, r_out,
        "stdout mismatch for input {:?}\nC stdout:    {:?}\nRust stdout: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        c_code, r_code,
        "exit code mismatch for input {:?}: C={:?} Rust={:?}",
        String::from_utf8_lossy(input),
        c_code,
        r_code
    );
}

#[test]
fn matches_zero() {
    assert_match(b"0\n");
}

#[test]
fn matches_positive_small() {
    assert_match(b"5\n");
}

#[test]
fn matches_positive_large() {
    assert_match(b"1000000\n");
}

#[test]
fn matches_negative() {
    assert_match(b"-42\n");
}

#[test]
fn matches_negative_large() {
    assert_match(b"-1000000\n");
}

#[test]
fn matches_explicit_plus_sign() {
    assert_match(b"+7\n");
}

#[test]
fn matches_leading_whitespace() {
    assert_match(b"   123\n");
}

#[test]
fn matches_leading_tabs_and_newlines() {
    assert_match(b"\t\n  42\n");
}

#[test]
fn matches_no_trailing_newline() {
    assert_match(b"99");
}

#[test]
fn matches_empty_input() {
    // scanf returns 0; C code falls through and prints driver(0) = 300.
    // Rust does the same: x stays 0.
    assert_match(b"");
}

#[test]
fn matches_only_whitespace() {
    // scanf hits EOF after consuming whitespace -> returns EOF, x stays 0.
    assert_match(b"   \n  \t\n");
}

#[test]
fn matches_int_max() {
    // INT_MAX = 2147483647. driver doubles it -> overflow.
    // C uses signed int with undefined-on-overflow semantics in theory, but
    // on x86 it just wraps. Rust uses wrapping_mul/wrapping_add to match.
    assert_match(b"2147483647\n");
}

#[test]
fn matches_int_min() {
    assert_match(b"-2147483648\n");
}

#[test]
fn matches_value_that_overflows_doubling() {
    // 2 * 1500000000 = 3000000000 > INT_MAX -> wraps.
    assert_match(b"1500000000\n");
}

#[test]
fn matches_value_that_overflows_after_add() {
    // 2 * 1073741674 = 2147483348; +300 = 2147483648 -> wraps to INT_MIN.
    assert_match(b"1073741674\n");
}

#[test]
fn matches_extra_trailing_garbage() {
    // scanf only reads the leading integer; rest is ignored.
    assert_match(b"42 hello world\n");
}

#[test]
fn matches_negative_zero() {
    assert_match(b"-0\n");
}

#[test]
fn matches_plus_zero() {
    assert_match(b"+0\n");
}
