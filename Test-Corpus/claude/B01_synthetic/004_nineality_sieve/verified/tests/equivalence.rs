// Integration tests comparing the C reference binary to the Rust port.
//
// The C source produces an executable (no library symbols), so we test
// equivalence by running both binaries with the same arguments and
// comparing stdout, stderr, and exit code byte-for-byte.

use std::path::PathBuf;
use std::process::Command;

fn c_binary() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    p
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, args: &[&str]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", bin, e));
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(args: &[&str]) {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.exists(), "C binary missing at {:?} (run cmake build)", c);
    assert!(r.exists(), "Rust binary missing at {:?}", r);

    let (c_out, c_err, c_code) = run(&c, args);
    let (r_out, r_err, r_code) = run(&r, args);

    assert_eq!(
        c_out, r_out,
        "stdout mismatch for args {:?}\n C: {:?}\n R: {:?}",
        args,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        c_err, r_err,
        "stderr mismatch for args {:?}\n C: {:?}\n R: {:?}",
        args,
        String::from_utf8_lossy(&c_err),
        String::from_utf8_lossy(&r_err)
    );
    assert_eq!(
        c_code, r_code,
        "exit code mismatch for args {:?}: C={:?} R={:?}",
        args, c_code, r_code
    );
}

#[test]
fn no_args_error() {
    assert_match(&[]);
}

#[test]
fn too_many_args_error() {
    assert_match(&["1", "2"]);
}

#[test]
fn three_args_error() {
    assert_match(&["1", "2", "3"]);
}

#[test]
fn non_integer_arg_error() {
    assert_match(&["abc"]);
}

#[test]
fn empty_string_arg_error() {
    assert_match(&[""]);
}

#[test]
fn whitespace_only_arg_error() {
    assert_match(&["   "]);
}

#[test]
fn just_sign_arg_error() {
    // strtol with "+" or "-" sets end == start (no digits).
    assert_match(&["+"]);
    assert_match(&["-"]);
}

#[test]
fn start_at_zero() {
    assert_match(&["0"]);
}

#[test]
fn start_at_one() {
    assert_match(&["1"]);
}

#[test]
fn start_at_five() {
    assert_match(&["5"]);
}

#[test]
fn start_at_eight() {
    assert_match(&["8"]);
}

#[test]
fn start_at_nine() {
    // Already ends in 9: prints once and exits.
    assert_match(&["9"]);
}

#[test]
fn start_at_ten() {
    assert_match(&["10"]);
}

#[test]
fn start_at_negative_three() {
    // -3, -2, -1, 0, ..., 9
    assert_match(&["-3"]);
}

#[test]
fn start_at_negative_eleven() {
    assert_match(&["-11"]);
}

#[test]
fn start_at_negative_one() {
    assert_match(&["-1"]);
}

#[test]
fn start_at_negative_nine() {
    // -9 ends in 9 (since -9 % 10 == -9 in C99 truncated semantics, but
    // careful: the C check is `val % 10 == 9`, which is FALSE for -9).
    // The Rust must mirror that.
    assert_match(&["-9"]);
}

#[test]
fn start_at_negative_nineteen() {
    assert_match(&["-19"]);
}

#[test]
fn start_with_plus_sign() {
    assert_match(&["+5"]);
}

#[test]
fn start_with_leading_zeros() {
    // strtol base 10 accepts leading zeros without changing base.
    assert_match(&["007"]);
}

#[test]
fn start_with_leading_whitespace() {
    assert_match(&["   3"]);
}

#[test]
fn trailing_garbage_is_ignored() {
    // strtol parses "5" and stops; end != start so it's accepted.
    assert_match(&["5xyz"]);
}

#[test]
fn leading_garbage_is_error() {
    assert_match(&["x5"]);
}
