use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

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

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust.stdout, c.stdout,
        "{case}: stdout differs for input {input:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case}: stderr differs for input {input:?}"
    );
    assert_eq!(
        rust.status, c.status,
        "{case}: exit status differs for input {input:?}"
    );
}

#[test]
fn empty_input_hits_both_fgets_error_paths() {
    assert_matches_c("empty input", b"");
}

#[test]
fn single_item_hits_the_second_fgets_error_path() {
    assert_matches_c("single item", b"2\n");
}

#[test]
fn final_item_without_a_newline_is_still_a_successful_read() {
    assert_matches_c("unterminated final item", b"2\n4");
}

#[test]
fn two_items_take_both_success_paths() {
    assert_matches_c("two items", b"2\n4\n");
}

#[test]
fn blank_lines_are_successful_reads_that_convert_to_zero() {
    assert_matches_c("blank lines", b"\n\n");
}

#[test]
fn zero_and_negative_zero_take_the_guarded_else_path() {
    assert_matches_c("zero then negative zero", b"0\n-0\n");
}

#[test]
fn values_on_both_sides_of_the_guard_threshold() {
    assert_matches_c("guard threshold", b"0.000001\n0.0000011\n");
}

#[test]
fn division_truncates_for_positive_and_negative_values() {
    assert_matches_c("signed truncation", b"3\n-6\n");
}

#[test]
fn invalid_input_converts_to_zero() {
    assert_matches_c("invalid atof input", b"not-a-number\nstill-invalid\n");
}

#[test]
fn nan_and_infinity_follow_c_comparisons_and_casts() {
    assert_matches_c("nan and infinity", b"nan\ninf\n");
    assert_matches_c("infinity and nan", b"inf\nnan\n");
}

#[test]
fn tiny_values_exercise_out_of_range_integer_conversion() {
    assert_matches_c("positive overflow", b"2\n1e-40\n");
    assert_matches_c("negative overflow", b"2\n-1e-40\n");
}

#[test]
fn largest_finite_float_is_accepted() {
    assert_matches_c("largest finite float", b"3.4028235e38\n3.4028235e38\n");
}

#[test]
fn eighteen_payload_bytes_and_newline_fit_in_one_fgets_call() {
    assert_matches_c("18-byte payload plus newline", b"2.0000000000000000\n4\n");
}

#[test]
fn nineteen_payload_bytes_fill_one_fgets_call() {
    assert_matches_c("19-byte payload at EOF", b"2.00000000000000000");
}

#[test]
fn newline_after_nineteen_payload_bytes_becomes_the_second_read() {
    assert_matches_c("19-byte payload", b"2.00000000000000000\n4\n");
}

#[test]
fn overlong_first_line_spills_into_the_second_read() {
    assert_matches_c("overlong line", b"12345678901234567890\n7\n");
}

#[test]
fn embedded_nul_terminates_atof_but_not_fgets() {
    assert_matches_c("embedded NUL", b"2\0ignored\n4\0ignored\n");
}
