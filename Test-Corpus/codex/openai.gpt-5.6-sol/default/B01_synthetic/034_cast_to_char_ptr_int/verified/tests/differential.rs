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
        .expect("C driver is missing; build it with CMake before running the tests")
}

fn assert_matches_c(input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "stdout differs for input {input:?}");
    assert_eq!(rust.stderr, c.stderr, "stderr differs for input {input:?}");
    assert_eq!(
        rust.status, c.status,
        "exit status differs for input {input:?}"
    );
}

macro_rules! differential_case {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            assert_matches_c($input);
        }
    };
}

differential_case!(empty_input, b"");
differential_case!(whitespace_only, b" \n\t");
differential_case!(single_item, b"42");
differential_case!(zero, b"0\n");
differential_case!(maximum_int, b"2147483647\n");
differential_case!(minimum_int, b"-2147483648\n");
differential_case!(leading_whitespace_across_lines, b"\n\t 123\n");
differential_case!(leading_plus_and_zeroes, b"+00017\n");
differential_case!(invalid_item, b"not-a-number\n");
differential_case!(sign_without_digits, b"+\n");
differential_case!(positive_overflow, b"2147483648\n");
differential_case!(negative_overflow, b"-2147483649\n");
differential_case!(
    overflow_beyond_machine_word,
    b"9999999999999999999999999999999999999999\n"
);
differential_case!(numeric_prefix, b"123abc\n");
differential_case!(trailing_item_is_ignored, b"7 99\n");
