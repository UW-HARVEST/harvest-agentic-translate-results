// Integration tests comparing the C reference binary against the Rust
// translation. Both produce only an executable (no library functions to
// export), so the function under test is `main` and we exercise it by
// invoking each binary as a subprocess and comparing stdout, stderr, and
// exit status byte-for-byte.

use std::path::PathBuf;
use std::process::Command;

fn c_binary() -> PathBuf {
    // c_src/build/driver, relative to the crate manifest dir.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    assert!(p.exists(), "C binary not built at {:?}", p);
    p
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(bin: &PathBuf, args: &[&str]) -> RunResult {
    let out = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to spawn binary");
    RunResult {
        code: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn check(args: &[&str]) {
    let c = run(&c_binary(), args);
    let r = run(&rust_binary(), args);
    assert_eq!(
        c.code, r.code,
        "exit-code mismatch for args {:?}: C={:?} Rust={:?}",
        args, c.code, r.code
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch for args {:?}:\n  C   = {:?}\n  Rust= {:?}",
        args,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr mismatch for args {:?}:\n  C   = {:?}\n  Rust= {:?}",
        args,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
}

// ---------- Argument-count error paths ----------

#[test]
fn no_args_error() {
    check(&[]);
}

#[test]
fn too_many_args_error() {
    check(&["abcdef", "1", "2", "3"]);
}

// ---------- Single-arg path: stop defaults to len, start defaults to 0 ----------

#[test]
fn single_arg_simple() {
    check(&["hello"]);
}

#[test]
fn single_arg_empty_string() {
    check(&[""]);
}

#[test]
fn single_arg_one_char() {
    check(&["x"]);
}

#[test]
fn single_arg_long_string() {
    check(&["the quick brown fox jumps over the lazy dog"]);
}

// ---------- Two-arg path: only start specified ----------

#[test]
fn two_args_start_zero() {
    check(&["hello", "0"]);
}

#[test]
fn two_args_start_middle() {
    check(&["hello", "2"]);
}

#[test]
fn two_args_start_equals_len() {
    // start == len is allowed by `start > len` check (not strictly greater).
    // C will then printf("%.*s", len - start, argv[1] + start) = print "" + "\n".
    check(&["hello", "5"]);
}

#[test]
fn two_args_start_off_end_error() {
    check(&["hello", "6"]);
}

#[test]
fn two_args_start_far_off_end_error() {
    check(&["hello", "100"]);
}

#[test]
fn two_args_start_not_integer_error() {
    check(&["hello", "abc"]);
}

#[test]
fn two_args_start_empty_error() {
    check(&["hello", ""]);
}

// ---------- Three-arg path ----------

#[test]
fn three_args_full_range() {
    check(&["hello", "0", "5"]);
}

#[test]
fn three_args_inner_slice() {
    check(&["hello", "1", "4"]);
}

#[test]
fn three_args_single_char() {
    check(&["hello", "2", "3"]);
}

#[test]
fn three_args_stop_off_end_error() {
    check(&["hello", "0", "6"]);
}

#[test]
fn three_args_stop_le_start_error_equal() {
    check(&["hello", "2", "2"]);
}

#[test]
fn three_args_stop_lt_start_error() {
    check(&["hello", "3", "1"]);
}

#[test]
fn three_args_long_string() {
    check(&["the quick brown fox", "4", "9"]);
}

#[test]
fn three_args_long_string_full() {
    check(&["the quick brown fox", "0", "19"]);
}

// ---------- Edge cases around strtol / numeric parsing ----------

#[test]
fn three_args_start_with_leading_whitespace() {
    // strtol skips leading whitespace.
    check(&["hello", "  1", "4"]);
}

#[test]
fn three_args_start_with_plus_sign() {
    check(&["hello", "+1", "4"]);
}

#[test]
fn three_args_start_with_trailing_garbage() {
    // strtol succeeds at "1" then stops; consumed > 0 so no error.
    check(&["hello", "1abc", "4"]);
}

#[test]
fn three_args_stop_with_trailing_garbage() {
    // The C check `end == argv[3]` is buggy (uses stale `end` from argv[2]),
    // so even a fully non-numeric stop slips past the parse-error check.
    // Then strtol returns 0, and `stop <= start` fires. The Rust port must
    // produce the exact same output sequence.
    check(&["hello", "1", "4xyz"]);
}

#[test]
fn three_args_stop_completely_nonnumeric() {
    // Same buggy path: parse error is NOT reported; strtol returns 0; the
    // `stop <= start` check fires.
    check(&["hello", "1", "abc"]);
}

#[test]
fn three_args_negative_start() {
    // Negative start: C casts int to size_t for `start > len` -> huge value
    // -> "off the end" error.
    check(&["hello", "-1", "3"]);
}

#[test]
fn three_args_negative_stop() {
    // start parses OK, then stop = -1; cast to size_t -> huge -> "off the end"
    check(&["hello", "1", "-1"]);
}

#[test]
fn empty_string_with_zero_start_zero_stop_error() {
    // stop <= start -> error
    check(&["", "0", "0"]);
}

#[test]
fn empty_string_with_start_zero_only() {
    check(&[""]);
}

#[test]
fn empty_string_with_start_zero_arg() {
    check(&["", "0"]);
}

// ---------- Whitespace & special characters in the source string ----------

#[test]
fn string_with_spaces() {
    check(&["a b c d e", "2", "7"]);
}

#[test]
fn string_with_tabs_newlines() {
    check(&["a\tb\nc", "1", "3"]);
}

// ---------- Binary-ish (high-byte) string content ----------

#[test]
fn string_with_high_bytes() {
    check(&["héllo", "0", "3"]);
}
