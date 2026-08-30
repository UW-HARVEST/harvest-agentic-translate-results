use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to collect output")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("C driver is missing; build it with cmake before running cargo test")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c_output = run(&c_binary(), input);
    let rust_output = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust_output.stdout, c_output.stdout,
        "{case}: stdout differs"
    );
    assert_eq!(
        rust_output.stderr, c_output.stderr,
        "{case}: stderr differs"
    );
    assert_eq!(
        rust_output.status, c_output.status,
        "{case}: exit status differs"
    );
}

#[test]
fn empty_and_whitespace_only_input() {
    assert_matches_c("empty input", b"");
    assert_matches_c("whitespace only", b" \t\r\n");
}

#[test]
fn one_item_and_values_across_lines() {
    assert_matches_c("single item", b"42\n");
    assert_matches_c("values across lines", b"1\n  -2\t3\r\n17");
}

#[test]
fn conversion_failure_before_and_after_items() {
    assert_matches_c("invalid first token", b"not-a-number 9\n");
    assert_matches_c("invalid token after values", b"5\n6 invalid\n7\n");
    assert_matches_c("numeric prefix before invalid suffix", b"12x 13\n");
}

#[test]
fn signed_limits_and_scanf_overflow() {
    assert_matches_c("signed integer limits", b"-2147483648 +2147483647\n");
    assert_matches_c("positive int overflow", b"2147483648\n");
    assert_matches_c("negative int overflow", b"-2147483649\n");
    assert_matches_c("far beyond int range", b"999999999999999999999999\n");
}

#[test]
fn exactly_one_hundred_items() {
    let input = (1..=100)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert_matches_c("exactly 100 items", input.as_bytes());
}

#[test]
fn input_after_the_hundredth_item_is_ignored() {
    let mut values = (1..=100)
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    values.extend(["999".to_owned(), "invalid".to_owned()]);
    let input = values.join("\n");
    assert_matches_c("more than 100 items", input.as_bytes());
}
