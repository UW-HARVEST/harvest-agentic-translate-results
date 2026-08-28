use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run_with_input(program: &Path, input: &[u8], args: &[&str]) -> Outcome {
    let mut child = Command::new(program)
        .args(args)
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

    let output = child.wait_with_output().expect("failed to wait for child");
    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    }
}

fn run_with_stdin(program: &Path, stdin: File) -> Outcome {
    let output = Command::new(program)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    Outcome {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    }
}

fn run_with_stdout(program: &Path, stdout: File, input: &[u8]) -> Outcome {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

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

fn assert_same(case: &str, c: Outcome, rust: Outcome) {
    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

fn compare(case: &str, input: &[u8]) {
    compare_with_args(case, input, &[]);
}

fn compare_with_args(case: &str, input: &[u8], args: &[&str]) {
    assert!(
        c_binary().is_file(),
        "C binary is missing; build c_src/build/driver first"
    );
    let c = run_with_input(&c_binary(), input, args);
    let rust = run_with_input(&rust_binary(), input, args);
    assert_same(case, c, rust);
}

#[test]
fn empty_input() {
    compare("empty input", b"");
}

#[test]
fn single_item_inputs() {
    compare("single byte at EOF", b"x");
    compare("single newline", b"\n");
    compare("single line", b"x\n");
}

#[test]
fn multiple_lines_and_partial_final_line() {
    compare(
        "multiple lines and partial final line",
        b"first\n\nthird line\nlast",
    );
}

#[test]
fn fgets_buffer_boundaries() {
    let line_filling_buffer = [b'a'; 126].into_iter().chain([b'\n']).collect::<Vec<_>>();
    compare("126 bytes plus newline", &line_filling_buffer);

    let newline_after_boundary = [b'b'; 127].into_iter().chain([b'\n']).collect::<Vec<_>>();
    compare("127 bytes plus newline", &newline_after_boundary);

    compare("127 bytes at EOF", &[b'c'; 127]);
    compare("254 bytes at EOF", &[b'd'; 254]);
}

#[test]
fn embedded_nuls_follow_fputs_semantics() {
    compare(
        "embedded NULs",
        b"before\0hidden\nvisible\n\0whole line hidden\nafter",
    );

    let mut boundary_nul = vec![b'e'; 126];
    boundary_nul.extend_from_slice(b"\0tail\nnext\n");
    compare("NUL at fgets boundary", &boundary_nul);
}

#[test]
fn arbitrary_bytes_are_not_decoded() {
    compare(
        "non-UTF-8 and carriage return bytes",
        &[0xff, 0xfe, b'\r', b'\n', 0x80, b'\n'],
    );
}

#[test]
fn command_line_arguments_are_ignored() {
    compare_with_args(
        "ignored command-line arguments",
        b"argument probe\n",
        &["first", "--flag", "last"],
    );
}

#[test]
fn long_input_spans_many_reads() {
    let input = (0..32_768)
        .map(|index| match index % 131 {
            0 => b'\n',
            17 => 0,
            _ => b'a' + (index % 26) as u8,
        })
        .collect::<Vec<_>>();
    compare("long multi-chunk input", &input);
}

#[test]
fn read_error_matches_eof_path() {
    let bad_c_stdin = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("failed to open write-only stdin");
    let bad_rust_stdin = bad_c_stdin
        .try_clone()
        .expect("failed to clone write-only stdin");

    let c = run_with_stdin(&c_binary(), bad_c_stdin);
    let rust = run_with_stdin(&rust_binary(), bad_rust_stdin);
    assert_same("stdin read error", c, rust);
}

#[test]
fn write_error_has_identical_observable_result() {
    let c_stdout = OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("failed to open /dev/full");
    let rust_stdout = c_stdout.try_clone().expect("failed to clone /dev/full");

    let c = run_with_stdout(&c_binary(), c_stdout, b"write failure\n");
    let rust = run_with_stdout(&rust_binary(), rust_stdout, b"write failure\n");
    assert_same("stdout write error", c, rust);
}
