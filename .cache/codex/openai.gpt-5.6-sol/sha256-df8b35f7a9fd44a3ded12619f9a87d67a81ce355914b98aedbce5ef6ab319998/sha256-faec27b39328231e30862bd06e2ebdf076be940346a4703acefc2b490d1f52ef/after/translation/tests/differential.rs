use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_input(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("piped stdin was unavailable")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", binary.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", binary.display()))
}

fn run_with_read_error(binary: &Path) -> Output {
    let unreadable_stdin =
        File::open(env!("CARGO_MANIFEST_DIR")).expect("failed to open directory for stdin");

    Command::new(binary)
        .stdin(Stdio::from(unreadable_stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_same(case: &str, c: Output, rust: Output) {
    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input (EOF)", b""),
        ("blank line", b"\n"),
        ("zero items", b"0\n"),
        ("single item", b"1\n"),
        ("largest copy", b"99\n"),
        ("copy boundary", b"100\n"),
        ("above copy boundary", b"101\n"),
        ("EOF after an item", b"2"),
        ("only the first line is read", b"3\n9\n"),
        ("nonnumeric input converts to zero", b"not-a-number\n"),
        ("leading whitespace, sign, and suffix", b"\t +2suffix\n"),
        ("embedded NUL terminates atoi", b"2\0 99\n"),
        ("fgets stops after 13 bytes", b"00000000000009\n"),
        ("positive int overflow becomes negative", b"2147483648\n"),
        ("unsigned-width value truncates to zero", b"4294967296\n"),
        ("negative int overflow becomes positive", b"-2147483649\n"),
        ("negative copy count", b"-1\n"),
    ];

    for &(name, input) in cases {
        assert_same(
            name,
            run_with_input(&c_binary(), input),
            run_with_input(&rust_binary(), input),
        );
    }
}

#[test]
fn matches_c_when_stdin_read_fails() {
    assert_same(
        "stdin read error",
        run_with_read_error(&c_binary()),
        run_with_read_error(&rust_binary()),
    );
}
