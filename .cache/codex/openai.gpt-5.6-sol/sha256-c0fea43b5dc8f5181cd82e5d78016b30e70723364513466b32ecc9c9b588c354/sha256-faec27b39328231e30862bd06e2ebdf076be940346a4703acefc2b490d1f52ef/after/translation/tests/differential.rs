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
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to collect output")
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
        ("whitespace-only input", b" \t\r\n"),
        ("invalid token", b"not-an-integer\n"),
        ("plus sign without digits", b"+"),
        ("single zero", b"0\n"),
        ("single positive item", b"42\n"),
        ("single negative item", b"-42\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("one above maximum int", b"2147483648\n"),
        ("one below minimum int", b"-2147483649\n"),
        (
            "very large positive integer",
            b"999999999999999999999999999999\n",
        ),
        (
            "very large negative integer",
            b"-999999999999999999999999999999\n",
        ),
        ("leading newlines", b"\n\n17\n"),
        ("valid prefix with trailing junk", b"123xyz\n"),
        ("multiple items", b"7 99\n"),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}
