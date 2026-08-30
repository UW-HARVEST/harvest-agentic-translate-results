use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn run_driver(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to collect output")
}

fn assert_matches_c(name: &str, input: &[u8]) {
    let c = run_driver(&c_driver(), input);
    let rust = run_driver(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
    assert_eq!(rust.status, c.status, "{name}: exit status differs");
}

#[test]
fn matches_c_for_all_input_classes() {
    let ninety_eight_spaces_then_digit = format!("{}7\n", " ".repeat(98));
    let ninety_nine_spaces_then_digit = format!("{}7\n", " ".repeat(99));
    let long_valid_prefix = format!("7{}", "x".repeat(120));

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty_eof", b"".to_vec()),
        ("blank_line", b"\n".to_vec()),
        ("whitespace_only", b" \t\r\x0b\x0c\n".to_vec()),
        ("non_numeric", b"house\n".to_vec()),
        ("plus_without_digits", b"+\n".to_vec()),
        ("minus_without_digits", b"-suffix\n".to_vec()),
        ("single_item", b"0\n".to_vec()),
        ("positive", b"3\n".to_vec()),
        ("negative", b"-4\n".to_vec()),
        ("explicit_plus", b"+12\n".to_vec()),
        ("leading_whitespace", b" \t\r\x0b\x0c-7\n".to_vec()),
        ("valid_prefix", b"15 bedrooms\n".to_vec()),
        ("second_line_ignored", b"2\n999\n".to_vec()),
        ("no_trailing_newline", b"8".to_vec()),
        ("embedded_nul_before_digits", b"\x009\n".to_vec()),
        ("embedded_nul_after_digits", b"6\0ignored\n".to_vec()),
        ("int_max", i32::MAX.to_string().into_bytes()),
        ("int_min", i32::MIN.to_string().into_bytes()),
        ("above_int_max", b"2147483648\n".to_vec()),
        ("below_int_min", b"-2147483649\n".to_vec()),
        ("long_max_but_not_int", b"9223372036854775807\n".to_vec()),
        ("long_positive_overflow", b"9223372036854775808\n".to_vec()),
        ("long_negative_overflow", b"-9223372036854775809\n".to_vec()),
        (
            "far_beyond_long_overflow",
            format!("{}\n", "9".repeat(120)).into_bytes(),
        ),
        (
            "digit_at_fgets_limit",
            ninety_eight_spaces_then_digit.into_bytes(),
        ),
        (
            "digit_beyond_fgets_limit",
            ninety_nine_spaces_then_digit.into_bytes(),
        ),
        ("long_valid_prefix", long_valid_prefix.into_bytes()),
    ];

    for (name, input) in cases {
        assert_matches_c(name, &input);
    }
}
