use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/driver")
}

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write input to {}: {error}", binary.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", binary.display()))
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(&rust_binary(), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn empty_input() {
    assert_matches_c("empty input", b"");
}

#[test]
fn whitespace_only_input() {
    assert_matches_c("whitespace-only input", b" \n\t\r");
}

#[test]
fn non_numeric_input() {
    assert_matches_c("non-numeric conversion failure", b"not-a-number\n");
}

#[test]
fn zero_items() {
    assert_matches_c("zero items", b"0\n");
}

#[test]
fn negative_item_count() {
    assert_matches_c("negative item count", b"-7\n");
}

#[test]
fn minimum_i32_item_count() {
    assert_matches_c("minimum i32 item count", b"-2147483648\n");
}

#[test]
fn single_item() {
    assert_matches_c("single item", b"1\n");
}

#[test]
fn multiple_items() {
    assert_matches_c("multiple items", b"5\n");
}

#[test]
fn larger_positive_sequence() {
    assert_matches_c("larger positive sequence", b"64\n");
}

#[test]
fn scanf_skips_whitespace_across_lines() {
    assert_matches_c("scanf skips whitespace across lines", b"\n\t 3\n");
}

#[test]
fn scanf_accepts_a_leading_plus_and_ignores_trailing_text() {
    assert_matches_c(
        "scanf accepts a leading plus and ignores trailing text",
        b"+2 trailing text\n",
    );
}

#[test]
fn scanf_accepts_a_numeric_prefix() {
    assert_matches_c("scanf accepts a numeric prefix", b"2x\n");
}

#[test]
fn scanf_positive_overflow_matches_the_c_runtime() {
    assert_matches_c("scanf positive overflow", b"2147483648\n");
}

#[test]
fn scanf_negative_overflow_matches_the_c_runtime() {
    assert_matches_c("scanf negative overflow", b"-4294967296\n");
}
