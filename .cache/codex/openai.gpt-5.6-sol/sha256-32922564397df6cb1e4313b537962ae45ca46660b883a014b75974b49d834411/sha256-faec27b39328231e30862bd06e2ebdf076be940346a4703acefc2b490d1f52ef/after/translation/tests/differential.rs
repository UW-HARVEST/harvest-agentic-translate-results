use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Output};

const C_DRIVER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/build/driver");
const RUST_DRIVER: &str = env!("CARGO_BIN_EXE_driver");

fn run(program: &str, arguments: &[&[u8]]) -> Output {
    let mut command = Command::new(program);
    command.args(arguments.iter().map(|argument| OsStr::from_bytes(argument)));
    command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {program}: {error}"))
}

fn assert_matches_c(case: &str, arguments: &[&[u8]]) {
    let c = run(C_DRIVER, arguments);
    let rust = run(RUST_DRIVER, arguments);

    assert_eq!(c.stdout, rust.stdout, "{case}: stdout differs");
    assert_eq!(c.stderr, rust.stderr, "{case}: stderr differs");
    assert_eq!(c.status, rust.status, "{case}: exit status differs");
}

#[test]
fn missing_arguments() {
    assert_matches_c("empty input", &[]);
    assert_matches_c("single item", &[b"7"]);
}

#[test]
fn ordinary_and_ignored_arguments() {
    assert_matches_c("two positive integers", &[b"12", b"30"]);
    assert_matches_c("negative integer", &[b"-12", b"5"]);
    assert_matches_c("extra item is ignored", &[b"1", b"2", b"999"]);
}

#[test]
fn empty_and_non_numeric_arguments_convert_to_zero() {
    assert_matches_c("two empty strings", &[b"", b""]);
    assert_matches_c("no leading digits", &[b"word", b"!"]);
    assert_matches_c("sign without digits", &[b"+", b"-"]);
}

#[test]
fn atoi_whitespace_sign_and_terminator_paths() {
    assert_matches_c(
        "leading C-locale whitespace",
        &[b" \t\n\x0b\x0c\r+17", b"\t-4"],
    );
    assert_matches_c("digit scan terminates", &[b"123xyz", b"-9.5"]);
    assert_matches_c("whitespace after sign is not skipped", &[b"+ 8", b"-\t3"]);
}

#[test]
fn integer_boundaries_and_wrapping_addition() {
    assert_matches_c("maximum values", &[b"2147483647", b"2147483647"]);
    assert_matches_c("minimum values", &[b"-2147483648", b"-2147483648"]);
    assert_matches_c("positive addition overflow", &[b"2147483647", b"1"]);
    assert_matches_c("negative addition overflow", &[b"-2147483648", b"-1"]);
}

#[test]
fn atoi_truncation_and_overflow() {
    assert_matches_c("one above int maximum", &[b"2147483648", b"0"]);
    assert_matches_c("one below int minimum", &[b"-2147483649", b"0"]);
    assert_matches_c("maximum signed long", &[b"9223372036854775807", b"0"]);
    assert_matches_c("minimum signed long", &[b"-9223372036854775808", b"0"]);
    assert_matches_c(
        "positive overflow beyond signed long",
        &[b"999999999999999999999999999999999", b"0"],
    );
    assert_matches_c(
        "negative overflow beyond signed long",
        &[b"-999999999999999999999999999999999", b"0"],
    );
}

#[test]
fn non_utf8_arguments() {
    assert_matches_c("non-UTF-8 terminates digit scan", &[b"42\xff", b"1"]);
}
