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
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", executable.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", executable.display()))
}

fn c_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_executable(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn matches_c_for_every_input_class() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace then EOF", b" \n\t"),
        ("single nonzero item", b"1\n"),
        ("zero", b"0\n"),
        ("negative item", b"-1\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("positive int overflow", b"2147483648\n"),
        ("negative int overflow", b"-2147483649\n"),
        ("overflow truncating to zero", b"4294967296\n"),
        (
            "far positive overflow",
            b"99999999999999999999999999999999999999\n",
        ),
        (
            "far negative overflow",
            b"-99999999999999999999999999999999999999\n",
        ),
        ("invalid token", b"not-an-int\n"),
        ("sign without digits", b"+\n"),
        ("valid item after invalid token", b"x 1\n"),
        ("integer across newlines", b"\n\n9\n"),
        ("zero across newlines", b"\n\n0\n"),
        ("trailing item ignored", b"1 0\n"),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}
