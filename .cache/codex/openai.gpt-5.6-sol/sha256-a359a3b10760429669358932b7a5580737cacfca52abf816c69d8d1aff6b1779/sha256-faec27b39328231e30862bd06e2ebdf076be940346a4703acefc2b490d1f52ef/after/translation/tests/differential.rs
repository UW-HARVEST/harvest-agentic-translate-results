use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}

fn run(executable: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", executable.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    let output = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    }
}

fn c_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate has no parent directory")
        .join("c_src/build/driver")
}

fn assert_matches_c(name: &str, input: &[u8]) {
    let c = run(&c_executable(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{name}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{name}: stderr differs");
    assert_eq!(rust.status, c.status, "{name}: exit status differs");
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace then eof", b" \t\r\n"),
        ("conversion failure", b"not-a-number\n"),
        ("sign-only conversion failure", b"+\n"),
        ("zero", b"0\n"),
        ("explicit positive sign", b"+17\n"),
        ("single item", b"1\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("positive overflow", b"2147483648\n"),
        ("negative overflow", b"-2147483649\n"),
        ("extreme positive overflow", b"999999999999999999999999999999\n"),
        ("extreme negative overflow", b"-999999999999999999999999999999\n"),
        ("leading whitespace across lines", b"\n\n\t 42\n"),
        ("partial numeric token", b"12xyz\n"),
        ("trailing item is ignored", b"-7\n999\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}
