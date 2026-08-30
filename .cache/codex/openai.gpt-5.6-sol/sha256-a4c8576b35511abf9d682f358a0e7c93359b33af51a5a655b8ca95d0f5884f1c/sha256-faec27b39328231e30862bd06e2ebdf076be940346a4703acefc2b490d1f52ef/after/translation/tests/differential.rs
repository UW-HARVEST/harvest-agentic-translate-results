use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(program: &Path, input: &[u8]) -> Output {
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
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("C driver is missing; build it with cmake before running tests")
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

macro_rules! differential_case {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            assert_matches_c(stringify!($name), $input);
        }
    };
}

// scanf conversion-count classes and preserved initializer values.
differential_case!(empty_input, b"");
differential_case!(whitespace_only, b" \t\n\r\x0b\x0c");
differential_case!(single_item_rejected_x, b"0");
differential_case!(single_item_accepted_x, b"1");
differential_case!(two_items, b"1 2");
differential_case!(three_items_maximum_consumed, b"1 2 3");

// Every ordered multi_stage validation branch and its success return.
differential_case!(x_validation_failure, b"0 2 3\n");
differential_case!(y_validation_failure, b"1 0 3\n");
differential_case!(z_validation_failure, b"1 2 0\n");
differential_case!(all_valid, b"1 2 3\n");

// Conversion failures at each scanf destination.
differential_case!(invalid_first_conversion, b"invalid 2 3\n");
differential_case!(invalid_second_conversion, b"1 invalid 3\n");
differential_case!(invalid_third_conversion, b"1 2 invalid\n");
differential_case!(numeric_prefix_in_first_field, b"1x 2 3\n");
differential_case!(numeric_prefix_in_second_field, b"1 2x 3\n");
differential_case!(numeric_prefix_in_third_field, b"1 2 3x\n");
differential_case!(sign_without_digits_first, b"+ 2 3\n");
differential_case!(sign_without_digits_second, b"1 - 3\n");
differential_case!(sign_without_digits_third, b"1 2 +\n");
differential_case!(nul_stops_conversion, b"1 \0 3\n");

// scanf whitespace behavior and input left unread after the third conversion.
differential_case!(values_across_newlines, b"\n1\n2\n3\n");
differential_case!(all_c_whitespace_classes, b"\t1\r2\x0b3\x0c");
differential_case!(extra_integer_is_ignored, b"1 2 3 4\n");
differential_case!(extra_invalid_text_is_ignored, b"1 2 3 trailing\n");
differential_case!(explicit_signs_and_leading_zeroes, b"+01 +002 +0003\n");

// Integer boundaries, narrowing to int, and strtol-style overflow behavior.
differential_case!(signed_int_maximum, b"2147483647 2 3\n");
differential_case!(signed_int_minimum, b"-2147483648 2 3\n");
differential_case!(
    positive_narrowing_wraps_each_value,
    b"4294967297 4294967298 4294967299\n"
);
differential_case!(
    negative_narrowing_wraps_each_value,
    b"-4294967295 -4294967294 -4294967293\n"
);
differential_case!(signed_long_maximum, b"9223372036854775807 2 3\n");
differential_case!(signed_long_minimum, b"-9223372036854775808 2 3\n");
differential_case!(
    positive_decimal_overflow,
    b"999999999999999999999999999999999999 2 3\n"
);
differential_case!(
    negative_decimal_overflow,
    b"-999999999999999999999999999999999999 2 3\n"
);
