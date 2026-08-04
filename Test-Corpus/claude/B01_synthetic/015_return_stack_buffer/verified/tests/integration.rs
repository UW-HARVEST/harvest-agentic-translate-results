// Integration test: compare the C reference implementation's executable
// output against the Rust port's executable output, byte-for-byte.
//
// Note: This project is a *binary* (not a library) with no FFI surface
// (no `[features]`, no `[lib]`, no `#[no_mangle]` exports, and the C side
// is `add_executable(...)` rather than a shared library). Therefore the
// libloading-based comparison described in the harness instructions does
// not apply: there are no exported symbols to load. Instead we compare
// the observable behavior at the only public interface this program has,
// stdin -> stdout/exit_code.

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
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    // It points to the just-built binary for the package's [[bin]].
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(input).expect("write input");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    (out.stdout, out.stderr, out.status.code())
}

fn compare_for_input(input: &[u8]) {
    assert!(
        c_binary().exists(),
        "C binary not built at {:?}. Run cmake build first.",
        c_binary()
    );

    let (c_stdout, _c_stderr, c_code) = run(&c_binary(), input);
    let (r_stdout, _r_stderr, r_code) = run(&rust_binary(), input);

    assert_eq!(
        c_stdout, r_stdout,
        "stdout differs for input {:?}\n C: {:?}\n R: {:?}",
        input, c_stdout, r_stdout
    );
    assert_eq!(c_code, r_code, "exit code differs for input {:?}", input);
}

#[test]
fn input_zero_takes_bad_branch() {
    // x == 0 -> bad() in both. The C bad() prints via printLine() which
    // dereferences a dangling stack pointer; in practice gcc/clang
    // produce zero output bytes here. The Rust port intentionally
    // produces zero bytes too. Compare byte-for-byte.
    compare_for_input(b"0\n");
}

#[test]
fn input_one_takes_good_branch() {
    compare_for_input(b"1\n");
}

#[test]
fn input_negative_one_is_truthy() {
    compare_for_input(b"-1\n");
}

#[test]
fn input_large_positive() {
    compare_for_input(b"42\n");
}

#[test]
fn input_large_negative() {
    compare_for_input(b"-2147483648\n");
}

#[test]
fn input_int_max() {
    compare_for_input(b"2147483647\n");
}

#[test]
fn input_zero_no_trailing_newline() {
    compare_for_input(b"0");
}

#[test]
fn input_one_no_trailing_newline() {
    compare_for_input(b"1");
}

#[test]
fn input_empty_eof() {
    // scanf returns EOF; x stays 0; bad() runs.
    compare_for_input(b"");
}

#[test]
fn input_non_numeric_is_zero() {
    // scanf("%d", ...) on "abc" matches nothing; x stays initialized to 0.
    compare_for_input(b"abc\n");
}

#[test]
fn input_leading_whitespace_then_one() {
    compare_for_input(b"   \t\n  1\n");
}

#[test]
fn input_leading_whitespace_then_zero() {
    compare_for_input(b"   \t\n  0\n");
}

#[test]
fn input_plus_sign_one() {
    compare_for_input(b"+1\n");
}

#[test]
fn input_zero_with_trailing_garbage() {
    compare_for_input(b"0xyz\n");
}

#[test]
fn input_one_with_trailing_garbage() {
    compare_for_input(b"1xyz\n");
}

#[test]
fn input_multiple_numbers_first_zero() {
    // scanf only reads one int; the rest is left unread.
    compare_for_input(b"0 1 2\n");
}

#[test]
fn input_multiple_numbers_first_one() {
    compare_for_input(b"1 0 0\n");
}
