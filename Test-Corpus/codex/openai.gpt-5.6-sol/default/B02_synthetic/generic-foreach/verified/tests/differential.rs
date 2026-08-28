use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

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
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", executable.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", executable.display()))
}

fn assert_cases_match(cases: &[(&str, &[u8])]) {
    let c_executable =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver");
    let rust_executable = Path::new(env!("CARGO_BIN_EXE_driver"));
    assert!(
        c_executable.is_file(),
        "C executable is missing; build it at {}",
        c_executable.display()
    );

    let mut mismatches = String::new();
    for (name, input) in cases {
        let c = run(&c_executable, input);
        let rust = run(rust_executable, input);

        let status_matches = c.status == rust.status;
        let stdout_matches = c.stdout == rust.stdout;
        let stderr_matches = c.stderr == rust.stderr;
        if status_matches && stdout_matches && stderr_matches {
            continue;
        }

        writeln!(
            mismatches,
            "\ncase {name:?}, input {input:?}\n\
             C status: {:?}\nRust status: {:?}\n\
             C stdout: {:?}\nRust stdout: {:?}\n\
             C stderr: {:?}\nRust stderr: {:?}",
            c.status.code(),
            rust.status.code(),
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&rust.stdout),
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr),
        )
        .expect("writing to a String cannot fail");
    }

    assert!(mismatches.is_empty(), "{mismatches}");
}

#[test]
fn every_menu_choice_matches() {
    assert_cases_match(&[
        ("integer demo", b"1\n"),
        ("double demo", b"2\n"),
        ("inventory demo", b"3\n"),
        ("order demo", b"4\n"),
        ("mixed demo", b"5\n"),
        ("maximum valid choice runs all demos", b"6\n"),
        ("exit", b"7\n"),
        ("all choices in one process", b"1\n2\n3\n4\n5\n6\n7\n"),
    ]);
}

#[test]
fn eof_and_error_paths_match() {
    assert_cases_match(&[
        ("empty input", b""),
        ("empty line", b"\n"),
        ("whitespace only", b" \t\r\n"),
        ("non-numeric input", b"not a number\n"),
        ("sign without digits", b"+\n"),
        ("zero is an invalid choice", b"0\n"),
        ("negative invalid choice", b"-1\n"),
        ("choice above valid range", b"8\n"),
        ("maximum signed integer", b"2147483647\n"),
        ("minimum signed integer", b"-2147483648\n"),
    ]);
}

#[test]
fn sscanf_integer_parsing_matches() {
    assert_cases_match(&[
        ("leading whitespace", b" \t+7\n"),
        ("trailing junk is accepted", b"7junk\n"),
        ("first integer wins", b"7 1\n"),
        ("hex prefix parses as decimal zero", b"0x7\n"),
        ("positive overflow", b"2147483648\n"),
        ("negative overflow", b"-2147483649\n"),
        (
            "many positive digits",
            b"99999999999999999999999999999999999999999999999999\n",
        ),
        (
            "many negative digits",
            b"-99999999999999999999999999999999999999999999999999\n",
        ),
        ("NUL terminates the parsed string", b"7\0ignored\n"),
        ("digits after NUL are not parsed", b"x\07\n"),
    ]);
}

#[test]
fn fgets_buffer_boundaries_match() {
    let line_254_then_exit = format!("{}\n7\n", "x".repeat(254));
    let line_255_then_exit = format!("{}\n7\n", "x".repeat(255));
    let line_256_then_exit = format!("{}\n7\n", "x".repeat(256));
    let choice_with_254_trailing_bytes = format!("7{}", "x".repeat(254));

    assert_cases_match(&[
        (
            "254 bytes plus newline fits in one read",
            line_254_then_exit.as_bytes(),
        ),
        (
            "255 bytes leaves newline for the next read",
            line_255_then_exit.as_bytes(),
        ),
        (
            "256 bytes spans two data reads",
            line_256_then_exit.as_bytes(),
        ),
        (
            "maximum 255-byte chunk beginning with exit",
            choice_with_254_trailing_bytes.as_bytes(),
        ),
        ("multiple commands on one line use only the first", b"1 7\n"),
        ("input without a final newline", b"1"),
    ]);
}

#[test]
fn remaining_parser_and_chunk_paths_match() {
    let long_line_continuation = format!("{}7\n", "x".repeat(255));
    let valid_choice_then_continuation = format!("1{}7\n", "x".repeat(254));

    assert_cases_match(&[
        ("vertical tab is whitespace", b"\x0b7\n"),
        ("form feed is whitespace", b"\x0c7\n"),
        ("carriage return without newline", b"\r7"),
        ("negative zero", b"-0\n"),
        ("two signs are invalid", b"--7\n"),
        ("mixed signs are invalid", b"+-7\n"),
        ("leading NUL prevents parsing", b"\07\n"),
        ("non-ASCII leading byte", b"\xff7\n"),
        ("positive 32-bit wraparound reaches choice 1", b"4294967297\n"),
        ("positive 32-bit wraparound reaches choice 7", b"4294967303\n"),
        ("negative 32-bit wraparound reaches choice 1", b"-4294967295\n"),
        ("negative 32-bit wraparound reaches choice 7", b"-4294967289\n"),
        (
            "next chunk of one physical line is parsed independently",
            long_line_continuation.as_bytes(),
        ),
        (
            "valid full chunk leaves another command",
            valid_choice_then_continuation.as_bytes(),
        ),
        ("repeated parse errors continue until EOF", b"x\n+\n\0\n"),
    ]);
}
