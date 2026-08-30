use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run(executable: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", executable.display()));

    child
        .stdin
        .take()
        .expect("piped stdin was unavailable")
        .write_all(input)
        .expect("failed to write process input");

    child.wait_with_output().expect("failed to collect output")
}

fn assert_matches_c(name: &str, input: &[u8]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_driver = manifest_dir.join("../c_src/build/driver");
    let rust_driver = Path::new(env!("CARGO_BIN_EXE_driver"));

    assert!(
        c_driver.is_file(),
        "C reference executable is missing; build it with cmake first"
    );

    let expected = run(&c_driver, input);
    let actual = run(rust_driver, input);

    assert_eq!(
        actual.stdout, expected.stdout,
        "{name}: stdout differs for input {input:?}"
    );
    assert_eq!(
        actual.stderr, expected.stderr,
        "{name}: stderr differs for input {input:?}"
    );
    assert_eq!(
        actual.status, expected.status,
        "{name}: exit status differs for input {input:?}"
    );
}

#[test]
fn eof_and_item_counts_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty", b""),
        ("whitespace_only", b" \t\r\n"),
        ("single_item", b"3"),
        ("two_items", b"3 7"),
        ("three_items", b"3 7 -1"),
        ("four_items", b"3 7 -1 -9"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn scanf_conversion_failures_match_at_every_position() {
    let cases: &[(&str, &[u8])] = &[
        ("invalid_x", b"x 2 1 9"),
        ("invalid_y", b"1 x 1 9"),
        ("invalid_b", b"1 2 x 9"),
        ("invalid_z", b"1 2 1 x"),
        ("partial_numeric_token", b"1x 2 1 9"),
        ("sign_without_digits", b"+ 2 1 9"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn whitespace_and_trailing_input_match() {
    let cases: &[(&str, &[u8])] = &[
        ("items_across_lines", b"1\n2\n0\n-4\n"),
        ("mixed_whitespace", b"\t 2\r\n  5 \t -7\n +12"),
        ("trailing_items_ignored", b"3 7 1 9 99 invalid\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn bitfields_and_boolean_conversion_match() {
    let cases: &[(&str, &[u8])] = &[
        ("bitfield_maxima", b"3 7 0 0"),
        ("bitfield_wrap_once", b"4 8 1 0"),
        ("bitfield_mixed_truncation", b"6 13 2 0"),
        ("negative_boolean", b"0 0 -1 0"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn integer_limits_signedness_and_overflow_match() {
    let cases: &[(&str, &[u8])] = &[
        (
            "maximum_values",
            b"4294967295 4294967295 2147483647 2147483647",
        ),
        ("minimum_signed_values", b"0 0 -2147483648 -2147483648"),
        ("negative_unsigned_values", b"-1 -2 -1 -1"),
        (
            "one_past_positive_limits",
            b"4294967296 4294967297 2147483648 2147483648",
        ),
        (
            "one_past_negative_limit",
            b"0 0 -2147483649 -2147483649",
        ),
        (
            "far_positive_overflow",
            b"999999999999999999999999 999999999999999999999999 999999999999999999999999 999999999999999999999999",
        ),
        (
            "far_negative_overflow",
            b"-999999999999999999999999 -999999999999999999999999 -999999999999999999999999 -999999999999999999999999",
        ),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}
