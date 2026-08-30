use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to launch {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(
        Path::new(env!("CARGO_BIN_EXE_driver")),
        input,
    );

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn empty_input_uses_initialized_space() {
    assert_matches_c("empty input", b"");
}

#[test]
fn representative_input_classes_match() {
    let cases: &[(&str, &[u8])] = &[
        ("ordinary byte", b"A"),
        ("space", b" "),
        ("newline", b"\n"),
        ("nul", b"\0"),
        ("last positive result", &[0x7e]),
        ("signed positive boundary", &[0x7f]),
        ("signed negative boundary", &[0x80]),
        ("signed minus two", &[0xfe]),
        ("signed minus one", &[0xff]),
        ("extra bytes on one line", b"ABC"),
        ("extra bytes across lines", b"A\nB"),
    ];

    for (case, input) in cases {
        assert_matches_c(case, input);
    }
}

#[test]
fn every_possible_single_byte_input_matches() {
    for byte in u8::MIN..=u8::MAX {
        assert_matches_c(&format!("single byte 0x{byte:02x}"), &[byte]);
    }
}
