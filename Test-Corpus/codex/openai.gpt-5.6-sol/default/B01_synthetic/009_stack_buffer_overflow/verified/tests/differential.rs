use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to wait for child")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("build the C executable at c_src/build/driver before running tests")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("single item", b"0\n"),
        ("minimum index", b"0\n0\n"),
        ("maximum handled index", b"9\n9\n"),
        ("checked negative index", b"-1\n0\n"),
        ("checked upper out of bounds", b"10\n0\n"),
        ("unchecked negative index", b"0\n-1\n"),
        ("nonnumeric input", b"not-a-number\nalso-not-a-number\n"),
        ("leading whitespace and suffix", b" \t+9suffix\n0\n"),
        ("signed overflow truncation", b"2147483648\n0\n"),
        ("unsigned-width truncation", b"4294967305\n0\n"),
        ("embedded nul", b"9\0ignored\n0\n"),
        ("fgets size boundary", b"00000000000009\n"),
        ("line-oriented reads", b"1 2\n3\n"),
        ("trailing input ignored", b"1\n2\nunused\n"),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}
