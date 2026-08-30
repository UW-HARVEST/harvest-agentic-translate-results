use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate should have a parent directory")
        .join("c_src/build/driver")
}

fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", program.display()));

    child
        .stdin
        .as_mut()
        .expect("child stdin should be piped")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", program.display()));
    drop(child.stdin.take());

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", program.display()))
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_driver(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn ordinary_and_branching_inputs_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty", b""),
        ("single A", b"A"),
        ("single x", b"x"),
        ("single unrelated byte", b"!"),
        ("both targets repeated", b"BAxxAqA"),
        ("targets across newlines", b"A\nxx\nA"),
        ("non-UTF-8 bytes", b"\xffA\x80x"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn c_string_termination_matches() {
    let cases: &[(&str, &[u8])] = &[
        ("leading NUL", b"\0AAxxx"),
        ("embedded NUL", b"Ax\0AAxxx"),
        ("multiple NULs", b"A\0xxx\0AAA"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn read_boundaries_match() {
    let max_without_overwriting_terminator = vec![b'A'; 999];
    let maximum_read = vec![b'x'; 1000];
    let mut maximum_terminated = vec![b'A'; 999];
    maximum_terminated.push(0);
    let mut trailing_data = maximum_terminated.clone();
    trailing_data.extend_from_slice(b"xxxAAA");

    let cases = [
        ("999 non-NUL bytes", max_without_overwriting_terminator),
        ("1000 non-NUL bytes", maximum_read),
        ("1000 bytes ending in NUL", maximum_terminated),
        ("data after the 1000-byte read", trailing_data),
    ];

    for (name, input) in &cases {
        assert_matches_c(name, input);
    }
}
