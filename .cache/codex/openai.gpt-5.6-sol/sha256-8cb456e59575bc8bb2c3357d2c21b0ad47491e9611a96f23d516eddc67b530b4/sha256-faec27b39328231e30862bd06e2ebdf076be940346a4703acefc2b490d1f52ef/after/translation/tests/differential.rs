use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run_program(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write test input");

    child.wait_with_output().expect("failed to collect output")
}

fn assert_programs_match(case_name: &str, input: &[u8]) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_program = manifest_dir.join("../c_src/build/driver");
    let rust_program = PathBuf::from(env!("CARGO_BIN_EXE_driver"));

    assert!(
        c_program.is_file(),
        "C executable is missing; build it first at {}",
        c_program.display()
    );

    let expected = run_program(&c_program, input);
    let actual = run_program(&rust_program, input);

    assert_eq!(
        actual.stdout, expected.stdout,
        "{case_name}: stdout differs"
    );
    assert_eq!(
        actual.stderr, expected.stderr,
        "{case_name}: stderr differs"
    );
    assert_eq!(
        actual.status, expected.status,
        "{case_name}: exit status differs"
    );
}

#[test]
fn defaults_to_zero_when_scanf_cannot_read_an_integer() {
    for (name, input) in [
        ("empty input", b"".as_slice()),
        ("whitespace-only input", b" \n\t".as_slice()),
        ("invalid token", b"not-an-integer\n".as_slice()),
        ("sign without digits", b"+\n".as_slice()),
    ] {
        assert_programs_match(name, input);
    }
}

#[test]
fn accepts_single_integer_items() {
    for (name, input) in [
        ("single positive item", b"1\n".as_slice()),
        ("zero item", b"0\n".as_slice()),
        ("single negative item", b"-3\n".as_slice()),
        ("explicit positive sign", b"+5\n".as_slice()),
    ] {
        assert_programs_match(name, input);
    }
}

#[test]
fn handles_full_signed_integer_range() {
    for (name, input) in [
        ("maximum int", b"2147483647\n".as_slice()),
        ("minimum int", b"-2147483648\n".as_slice()),
    ] {
        assert_programs_match(name, input);
    }
}

#[test]
fn matches_scanf_tokenization() {
    for (name, input) in [
        ("integer after newlines", b"\n\n42\n".as_slice()),
        ("numeric prefix", b"12xyz\n".as_slice()),
        ("ignored trailing item", b"7 99\n".as_slice()),
    ] {
        assert_programs_match(name, input);
    }
}
