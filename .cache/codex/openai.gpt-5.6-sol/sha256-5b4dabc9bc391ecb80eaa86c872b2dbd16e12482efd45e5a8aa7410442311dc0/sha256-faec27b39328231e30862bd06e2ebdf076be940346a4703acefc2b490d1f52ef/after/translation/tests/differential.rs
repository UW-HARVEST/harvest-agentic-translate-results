use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}

fn run(binary: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("piped stdin is available")
        .write_all(input)
        .expect("input can be written");

    let output = child.wait_with_output().expect("process can be waited on");
    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    }
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn assert_matches_c(label: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{label}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{label}: stderr differs");
    assert_eq!(rust.status, c.status, "{label}: exit status differs");
}

#[test]
fn failed_conversions_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace only", b" \t\r\n"),
        ("non-numeric token", b"not-a-float\n"),
        ("sign without digits", b"+\n"),
        ("decimal point without digits", b".\n"),
        ("non-UTF-8 byte", b"\xff\n"),
    ];

    for (label, input) in cases {
        assert_matches_c(label, input);
    }
}

#[test]
fn finite_values_and_boundaries_match() {
    let cases: &[(&str, &[u8])] = &[
        ("zero", b"0\n"),
        ("single item", b"1\n"),
        ("negative zero", b"-0\n"),
        ("fraction", b"1.5\n"),
        ("leading multiline whitespace", b" \n\t-12.25\r\n"),
        ("maximum finite float", b"3.4028234663852886e38\n"),
        ("negative maximum finite float", b"-3.4028234663852886e38\n"),
        ("minimum normal float", b"1.17549435e-38\n"),
        ("minimum subnormal float", b"1.40129846e-45\n"),
    ];

    for (label, input) in cases {
        assert_matches_c(label, input);
    }
}

#[test]
fn range_and_special_values_match() {
    let cases: &[(&str, &[u8])] = &[
        ("positive overflow", b"3.4028236e38\n"),
        ("negative overflow", b"-3.4028236e38\n"),
        ("positive underflow", b"1e-50\n"),
        ("negative underflow", b"-1e-50\n"),
        ("infinity", b"inf\n"),
        ("negative infinity", b"-INFINITY\n"),
        ("not a number", b"nan\n"),
        ("negative not a number", b"-nan\n"),
        ("hexadecimal float", b"0x1.8p+1\n"),
    ];

    for (label, input) in cases {
        assert_matches_c(label, input);
    }
}

#[test]
fn token_and_stream_boundaries_match() {
    let cases: &[(&str, &[u8])] = &[
        ("numeric prefix", b"1trailing\n"),
        ("two items on one line", b"1 2\n"),
        ("two items on separate lines", b"1\n2\n"),
        ("incomplete exponent", b"1e\n"),
        ("embedded NUL after item", b"1\0ignored\n"),
    ];

    for (label, input) in cases {
        assert_matches_c(label, input);
    }
}
