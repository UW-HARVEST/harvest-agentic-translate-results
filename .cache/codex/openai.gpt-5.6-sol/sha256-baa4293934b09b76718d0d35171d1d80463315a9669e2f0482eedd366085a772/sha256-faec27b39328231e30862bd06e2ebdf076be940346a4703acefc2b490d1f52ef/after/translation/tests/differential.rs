use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run_with_input(program: &Path, input: &[u8]) -> Output {
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
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", program.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", program.display()))
}

#[test]
fn matches_c_for_every_input_class() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_program = manifest_dir.join("../c_src/build/driver");
    let rust_program = PathBuf::from(env!("CARGO_BIN_EXE_driver"));

    let cases: &[(&str, &[u8])] = &[
        ("empty_input", b""),
        ("whitespace_only_eof", b" \n\t"),
        ("conversion_failure_text", b"not-a-number\n"),
        ("conversion_failure_sign_only", b"+\n"),
        ("zero", b"0\n"),
        ("single_positive_item", b"1\n"),
        ("single_negative_item", b"-1\n"),
        ("maximum_int", b"2147483647\n"),
        ("minimum_int", b"-2147483648\n"),
        ("above_maximum_int", b"2147483648\n"),
        ("below_minimum_int", b"-2147483649\n"),
        ("leading_whitespace_across_newlines", b"\n\t\n42\n"),
        ("zero_with_ignored_trailing_item", b"0 1\n"),
        ("nonzero_with_ignored_trailing_text", b"7 trailing\n"),
    ];

    assert!(
        c_program.is_file(),
        "C reference executable is missing: run its CMake build first ({})",
        c_program.display()
    );

    for &(name, input) in cases {
        let expected = run_with_input(&c_program, input);
        let actual = run_with_input(&rust_program, input);

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
}
