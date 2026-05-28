// Integration test that compares the stdout of the C `driver` executable to
// the stdout of the Rust `driver` executable for a variety of stdin inputs.
//
// The C source builds an *executable* (`add_executable(driver src/main.c)` in
// c_src/CMakeLists.txt) and the Rust crate exposes only a `[[bin]]` target —
// neither side ships a shared library and there are no `[features]` defined,
// so the only way an external caller can interact with this code is by
// invoking the binary and reading its stdout. We therefore exercise the
// public surface of both implementations the same way.
//
// To run:
//   1. Build the C executable: see c_src/CMakeLists.txt build instructions.
//   2. Build the Rust executable: `cargo build --release`.
//   3. Run: `cargo test --release`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_driver_path() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_driver_path() -> PathBuf {
    // Tests run with `cargo test`, but the test runner sets CARGO_TARGET_TMPDIR
    // pointing inside `target/{profile}/...`. We resolve the binary relative
    // to the workspace target dir so we work with both `dev` and `release`.
    let manifest = workspace_root();
    // Try release first, then debug.
    let release = manifest.join("target").join("release").join("driver");
    if release.exists() {
        return release;
    }
    manifest.join("target").join("debug").join("driver")
}

fn run_with_stdin(bin: &PathBuf, input: &str) -> Vec<u8> {
    assert!(
        bin.exists(),
        "expected binary at {:?} but it does not exist; build it first",
        bin
    );
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn driver");
    {
        let stdin = child.stdin.as_mut().expect("failed to take stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("failed to write stdin");
    }
    let output = child
        .wait_with_output()
        .expect("failed to wait for driver");
    output.stdout
}

fn assert_match(input: &str) {
    let c_path = c_driver_path();
    let rust_path = rust_driver_path();

    let c_out = run_with_stdin(&c_path, input);
    let rust_out = run_with_stdin(&rust_path, input);

    if c_out != rust_out {
        panic!(
            "stdout mismatch for input {:?}\n--- C output ---\n{}\n--- Rust output ---\n{}\n",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
        );
    }
}

#[test]
fn input_zero() {
    assert_match("0\n");
}

#[test]
fn input_positive_small() {
    assert_match("3\n");
}

#[test]
fn input_positive_large() {
    assert_match("100\n");
}

#[test]
fn input_negative() {
    assert_match("-5\n");
}

#[test]
fn input_with_leading_whitespace() {
    assert_match("   42\n");
}

#[test]
fn input_explicit_plus_sign() {
    assert_match("+7\n");
}

#[test]
fn input_non_numeric() {
    // scanf("%d", &x) leaves x untouched on conversion failure; since x is
    // initialized to 0 we expect both implementations to behave as if 0.
    assert_match("abc\n");
}

#[test]
fn input_empty() {
    assert_match("");
}

#[test]
fn input_whitespace_only() {
    assert_match("   \n\t  ");
}

#[test]
fn input_trailing_garbage() {
    // scanf reads only the first integer; trailing characters are ignored.
    assert_match("17 hello world\n");
}

#[test]
fn input_one() {
    assert_match("1\n");
}

#[test]
fn input_negative_large() {
    assert_match("-12345\n");
}
