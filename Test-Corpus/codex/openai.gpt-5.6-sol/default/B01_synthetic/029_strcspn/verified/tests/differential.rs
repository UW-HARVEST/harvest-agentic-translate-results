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

    child.wait_with_output().expect("failed to collect output")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("C binary is missing; build it with CMake before running tests")
}

fn assert_matches_c(name: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust.stdout, c.stdout,
        "{name}: stdout mismatch for input {input:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{name}: stderr mismatch for input {input:?}"
    );
    assert_eq!(
        rust.status, c.status,
        "{name}: exit-status mismatch for input {input:?}"
    );
}

#[test]
fn eof_and_short_input_classes_match() {
    let cases: &[(&str, &[u8])] = &[
        ("immediate EOF", b""),
        ("single byte without newline", b"a"),
        ("single blank line", b"\n"),
        ("two blank lines", b"\n\n"),
        ("first line only", b"abc\n"),
        ("second line ends at EOF", b"abc\nx"),
        ("one item per line", b"a\nb\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn strcspn_result_classes_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty reject set", b"abc\n\n"),
        ("match at first byte", b"abc\nax\n"),
        ("match in the middle", b"abc\nbx\n"),
        ("match at final byte", b"abc\ncx\n"),
        ("no rejected byte", b"abc\nxyz\n"),
        ("duplicate rejected bytes", b"abc\nbb\n"),
        ("CRLF keeps carriage return", b"abc\r\n\rx\r\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn binary_input_classes_match() {
    let cases: &[(&str, &[u8])] = &[
        ("NUL starts first line", b"\0abc\nx\n"),
        ("NUL occurs inside first line", b"ab\0cd\nb\n"),
        ("NUL starts second line", b"abc\n\0x\n"),
        ("NUL occurs inside second line", b"abc\nx\0b\n"),
        ("non-UTF-8 bytes", b"\xff\x80a\n\x80\n"),
    ];

    for (name, input) in cases {
        assert_matches_c(name, input);
    }
}

#[test]
fn every_byte_value_matches() {
    for byte in u8::MIN..=u8::MAX {
        let first_line = [byte, b'\n', b'x', b'\n'];
        assert_matches_c(&format!("first-line byte 0x{byte:02x}"), &first_line);

        let second_line = [b'x', b'\n', byte, b'\n'];
        assert_matches_c(&format!("second-line byte 0x{byte:02x}"), &second_line);
    }
}

#[test]
fn fgets_buffer_boundaries_match() {
    let mut maximum_complete_lines = vec![b'a'; 98];
    maximum_complete_lines.push(b'\n');
    maximum_complete_lines.extend([b'b'; 98]);
    maximum_complete_lines.push(b'\n');
    assert_matches_c("maximum newline-terminated lines", &maximum_complete_lines);

    let mut first_read_fills_buffer = vec![b'a'; 99];
    first_read_fills_buffer.push(b'\n');
    first_read_fills_buffer.extend_from_slice(b"a\n");
    assert_matches_c(
        "99-byte first read leaves newline",
        &first_read_fills_buffer,
    );

    let mut overlong_first_line = vec![b'a'; 120];
    overlong_first_line.push(b'\n');
    overlong_first_line.extend_from_slice(b"z\n");
    assert_matches_c(
        "overlong first line supplies second read",
        &overlong_first_line,
    );

    let maximum_unterminated_read = vec![b'a'; 99];
    assert_matches_c(
        "maximum unterminated first read",
        &maximum_unterminated_read,
    );

    let mut maximum_second_read = b"abc\n".to_vec();
    maximum_second_read.extend([b'x'; 99]);
    maximum_second_read.push(b'\n');
    assert_matches_c("99-byte second read leaves newline", &maximum_second_read);

    let mut overlong_unterminated_input = vec![b'a'; 198];
    assert_matches_c(
        "both reads fill their buffers",
        &overlong_unterminated_input,
    );

    overlong_unterminated_input.push(b'a');
    assert_matches_c(
        "input remains after both full reads",
        &overlong_unterminated_input,
    );
}
