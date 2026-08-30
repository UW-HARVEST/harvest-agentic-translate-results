use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c_output = run(&c_driver(), input);
    let rust_output = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust_output.stdout, c_output.stdout,
        "{case}: stdout differs"
    );
    assert_eq!(
        rust_output.stderr, c_output.stderr,
        "{case}: stderr differs"
    );
    assert_eq!(
        rust_output.status, c_output.status,
        "{case}: exit status differs"
    );
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace then eof", b" \t\r\n"),
        ("single item", b"7"),
        ("maximum two items", b"7 3\n"),
        ("scanf crosses newlines", b"7\n3\n"),
        (
            "leading whitespace and trailing data",
            b" \t7\r\n3 trailing\n",
        ),
        ("first conversion fails", b"x 3\n"),
        ("second conversion fails", b"7 x\n"),
        ("second conversion reaches eof", b"7 \t\n"),
        ("sign without digits", b"7 +\n"),
        ("explicit signs and leading zeroes", b"+0005 -0001\n"),
        ("zero and all-one bit patterns", b"0 -1\n"),
        ("nonzero result", b"5 -1\n"),
        ("maximum and minimum int", b"2147483647 -2147483648\n"),
        ("minimum and maximum int", b"-2147483648 2147483647\n"),
        ("positive int overflow", b"2147483648 -2147483649\n"),
        ("negative int overflow", b"-2147483649 2147483648\n"),
        (
            "decimal magnitude exceeds wider integer types",
            b"999999999999999999999999999999 -999999999999999999999999999999\n",
        ),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}
