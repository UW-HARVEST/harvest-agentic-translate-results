use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn run(executable: &Path, input: &[u8]) -> Output {
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

    child.wait_with_output().expect("failed to collect output")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn input_and_conversion_branches_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace followed by eof", b" \t\r\n"),
        ("single item", b"7\n"),
        ("items split across lines", b"1\n2  \n\t3\n"),
        ("invalid first conversion", b"not-an-int\n"),
        ("invalid after valid items", b"2 3 nope 4\n"),
        ("valid integer followed by invalid suffix", b"12x 5\n"),
    ];

    for (case, input) in cases {
        assert_matches_c(case, input);
    }
}

#[test]
fn arithmetic_boundaries_match() {
    assert_matches_c(
        "signed integer arithmetic boundaries",
        b"0 -1 1 46340 46341 -46341 2147483647 -2147483648\n",
    );
}

#[test]
fn maximum_length_matches() {
    let input = (0..100)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    assert_matches_c("exactly 100 items", input.as_bytes());
}

#[test]
fn input_after_maximum_length_is_ignored() {
    let input = (0..=100)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert_matches_c("101st item is ignored", input.as_bytes());
}
