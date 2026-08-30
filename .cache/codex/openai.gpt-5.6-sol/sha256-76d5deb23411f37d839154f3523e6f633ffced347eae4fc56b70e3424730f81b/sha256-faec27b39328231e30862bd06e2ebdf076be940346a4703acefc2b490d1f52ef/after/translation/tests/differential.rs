use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
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

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    let stdout_matches = rust.stdout == c.stdout;
    let stderr_matches = rust.stderr == c.stderr;
    let status_matches = rust.status == c.status;

    assert!(
        stdout_matches && stderr_matches && status_matches,
        "{case} differed for input {input:?}\n\
         C:    status={:?}, stdout={:?}, stderr={:?}\n\
         Rust: status={:?}, stdout={:?}, stderr={:?}",
        c.status,
        c.stdout,
        c.stderr,
        rust.status,
        rust.stdout,
        rust.stderr,
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

differential_case!(empty_input_uses_both_defaults, b"");
differential_case!(whitespace_only_uses_both_defaults, b" \n\t");
differential_case!(invalid_first_item_uses_both_defaults, b"x 7\n");
differential_case!(single_item_leaves_second_default, b"7\n");
differential_case!(single_negative_item_leaves_second_default, b"-7");
differential_case!(invalid_second_item_leaves_second_default, b"7 x\n");
differential_case!(single_zero_item_is_not_division_by_zero, b"0\n");
differential_case!(positive_exact_division, b"8 2\n");
differential_case!(positive_division_with_remainder, b"8 3\n");
differential_case!(negative_numerator, b"-8 3\n");
differential_case!(negative_denominator, b"8 -3\n");
differential_case!(negative_numerator_and_denominator, b"-8 -3\n");
differential_case!(scanf_reads_across_newlines, b"8\n3\n");
differential_case!(trailing_items_are_ignored, b"8 3 99\n");
differential_case!(maximum_int, b"2147483647 1\n");
differential_case!(minimum_int, b"-2147483648 1\n");
differential_case!(one_above_maximum_int, b"2147483648 1\n");
differential_case!(one_below_minimum_int, b"-2147483649 1\n");
differential_case!(zero_denominator, b"1 0\n");
differential_case!(unrepresentable_quotient, b"-2147483648 -1\n");
