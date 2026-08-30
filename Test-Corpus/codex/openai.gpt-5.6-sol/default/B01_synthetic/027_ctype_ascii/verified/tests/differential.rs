use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_binary() -> &'static str {
    env!("CARGO_BIN_EXE_driver")
}

fn run(binary: impl AsRef<Path>, input: &[u8]) -> Output {
    let mut child = Command::new(binary.as_ref())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.as_ref().display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches(input: &[u8]) {
    let c = run(c_binary(), input);
    let rust = run(rust_binary(), input);

    assert_eq!(rust.stdout, c.stdout, "stdout differs for input {input:?}");
    assert_eq!(rust.stderr, c.stderr, "stderr differs for input {input:?}");
    assert_eq!(
        rust.status, c.status,
        "exit status differs for input {input:?}"
    );
}

#[test]
fn empty_input_matches() {
    assert_matches(b"");
}

#[test]
fn every_possible_single_byte_matches() {
    for byte in 0..=u8::MAX {
        assert_matches(&[byte]);
    }
}

#[test]
fn only_the_first_byte_is_consumed() {
    for input in [
        b"A\nz".as_slice(),
        b"\nA".as_slice(),
        b"\0ignored".as_slice(),
        b"\x7f\x00\xff".as_slice(),
        b"\xfftrailing input".as_slice(),
    ] {
        assert_matches(input);
    }
}
