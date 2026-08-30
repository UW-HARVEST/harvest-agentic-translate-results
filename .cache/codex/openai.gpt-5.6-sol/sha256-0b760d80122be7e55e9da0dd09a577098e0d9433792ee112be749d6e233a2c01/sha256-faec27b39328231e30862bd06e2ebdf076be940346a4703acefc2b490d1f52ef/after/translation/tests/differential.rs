use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_driver(path: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", path.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches_c(case_name: &str, input: &[u8]) {
    let c_output = run_driver(&c_driver(), input);
    let rust_output = run_driver(&rust_driver(), input);

    assert_eq!(
        rust_output.stdout, c_output.stdout,
        "{case_name}: stdout differs"
    );
    assert_eq!(
        rust_output.stderr, c_output.stderr,
        "{case_name}: stderr differs"
    );
    assert_eq!(
        rust_output.status, c_output.status,
        "{case_name}: exit status differs"
    );
}

#[test]
fn valid_values_match() {
    let cases: &[(&str, &[u8])] = &[
        ("single_item", b"1\n"),
        ("zero", b"0\n"),
        ("negative", b"-7\n"),
        ("int_max", b"2147483647\n"),
        ("int_min", b"-2147483648\n"),
        ("explicit_plus", b"+42\n"),
        ("leading_ascii_whitespace", b" \t\x0b\x0c\r+42\n"),
        ("numeric_prefix_before_junk", b"12xyz\n"),
        ("base_ten_stops_at_x", b"0x10\n"),
        ("no_trailing_newline", b"8"),
        ("embedded_nul_after_number", b"13\0junk\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn every_parse_error_condition_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("empty_input", b""),
        ("newline_only", b"\n"),
        ("whitespace_only", b" \t\r\n"),
        ("nonnumeric", b"house\n"),
        ("plus_without_digits", b"+\n"),
        ("minus_without_digits", b"-\n"),
        ("embedded_nul_before_number", b"\x0013\n"),
        ("above_int_max", b"2147483648\n"),
        ("below_int_min", b"-2147483649\n"),
        ("long_overflow", b"9223372036854775808\n"),
        ("negative_long_overflow", b"-9223372036854775809\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn fgets_boundaries_match() {
    let mut cases = vec![
        ("second_line_is_not_scanned", b"5\n999\n".to_vec()),
        ("number_only_on_second_line", b"\n5\n".to_vec()),
        ("exactly_99_leading_zeroes", vec![b'0'; 99]),
    ];

    let mut valid_prefix_at_limit = vec![b'x'; 100];
    valid_prefix_at_limit[0] = b'7';
    cases.push(("valid_prefix_before_99_byte_limit", valid_prefix_at_limit));

    let mut digits_after_limit = vec![b'x'; 99];
    digits_after_limit.extend_from_slice(b"7\n");
    cases.push(("digits_after_99_byte_limit", digits_after_limit));

    for (name, input) in cases {
        assert_matches_c(name, &input);
    }
}
