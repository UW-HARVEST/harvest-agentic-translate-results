use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(executable: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {}: {error}", executable.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches_c(input: &[u8]) {
    let c_executable = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver");
    let rust_executable = std::env::var_os("DIFFERENTIAL_RUST_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_driver")));

    assert!(
        c_executable.is_file(),
        "C reference executable is missing; build it with CMake first"
    );

    let c = run(&c_executable, input);
    let rust = run(&rust_executable, input);

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
differential_case!(whitespace_only, b" \t\n");
differential_case!(malformed_token, b"not-an-integer\n");
differential_case!(zero, b"0\n");
differential_case!(single_positive_item, b"1\n");
differential_case!(single_negative_item, b"-1\n");
differential_case!(maximum_int, b"2147483647\n");
differential_case!(minimum_int, b"-2147483648\n");
differential_case!(positive_overflow, b"2147483648\n");
differential_case!(negative_overflow, b"-2147483649\n");
differential_case!(leading_whitespace_and_plus_sign, b" \t+1\n");
differential_case!(integer_after_newline, b"\n1\n");
differential_case!(valid_integer_with_trailing_text, b"1 trailing\n");
