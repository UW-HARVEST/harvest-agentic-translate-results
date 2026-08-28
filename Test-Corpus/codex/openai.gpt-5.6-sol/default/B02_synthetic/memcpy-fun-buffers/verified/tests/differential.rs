use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
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
        .expect("failed to write test input");
    child.wait_with_output().expect("failed to collect output")
}

fn assert_matches(name: &str, input: impl AsRef<[u8]>) {
    let input = input.as_ref();
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        (rust.status, rust.stdout, rust.stderr),
        (c.status, c.stdout, c.stderr),
        "differential mismatch for {name}; input={:?}",
        String::from_utf8_lossy(input)
    );
}

fn repeated_buffer(length: usize, start: i32) -> String {
    let mut input = length.to_string();
    for index in 0..length {
        input.push(' ');
        input.push_str(&(start + index as i32).to_string());
    }
    input
}

#[test]
fn input_validation_and_truncation_match() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("nonnumeric operation", b"x"),
        ("missing buffer count", b"0"),
        ("nonnumeric buffer count", b"0 x"),
        ("zero buffer count", b"0 0"),
        ("negative buffer count", b"0 -1"),
        ("buffer count above maximum", b"0 101"),
        ("missing buffer length", b"1 1"),
        ("nonnumeric buffer length", b"1 1 x"),
        ("negative buffer length", b"1 1 -1"),
        ("buffer length above maximum", b"1 1 257"),
        ("missing first byte", b"1 1 1"),
        ("missing later byte", b"1 1 2 7"),
        ("nonnumeric byte", b"1 1 2 7 x"),
    ];

    for (name, input) in cases {
        assert_matches(name, input);
    }
}

#[test]
fn successful_operations_match() {
    let cases: &[(&str, &[u8])] = &[
        ("copy with scanf across lines", b"0\n2\n3\n1 2 300\n1\n9\n"),
        ("copy empty source", b"0 2 0 1 9"),
        ("reverse empty buffer", b"1 1 0"),
        ("reverse single byte", b"1 1 1 -1"),
        (
            "reverse bytes from signed integer extremes",
            b"1 1 2 -2147483648 2147483647",
        ),
        ("reverse multiple buffers", b"1 2 4 1 2 3 4 1 9"),
        ("merge with empty first buffer", b"2 2 0 3 5 6 7"),
        ("split at zero", b"3 1 3 5 6 7 0"),
        ("split in middle", b"3 1 4 5 6 7 8 2"),
        ("split at end", b"3 1 3 5 6 7 3"),
        ("interleave empty buffers", b"4 2 0 0"),
        ("interleave empty first buffer", b"4 2 0 2 8 9"),
        ("interleave empty second buffer", b"4 2 2 8 9 0"),
        ("interleave first buffer longer", b"4 2 3 1 2 3 1 9"),
        ("interleave second buffer longer", b"4 2 1 9 3 1 2 3"),
        ("rotate empty buffer", b"5 1 0 7"),
        ("rotate by zero", b"5 1 4 1 2 3 4 0"),
        ("rotate left", b"5 1 4 1 2 3 4 1"),
        ("rotate right with negative amount", b"5 1 4 1 2 3 4 -1"),
        ("rotate by exact length", b"5 1 4 1 2 3 4 4"),
        ("rotate by large negative amount", b"5 1 4 1 2 3 4 -9"),
        ("rotate by more than length", b"5 2 4 1 2 3 4 1 9 9"),
        ("checksum empty and nonempty", b"6 2 0 4 1 2 3 4"),
        ("trailing input is ignored", b"6 1 1 7 trailing tokens"),
    ];

    for (name, input) in cases {
        assert_matches(name, input);
    }
}

#[test]
fn operation_error_paths_match() {
    let merge_overflow = format!(
        "2 2 {} {}",
        repeated_buffer(128, 0),
        repeated_buffer(129, 128)
    );
    let interleave_overflow = format!(
        "4 2 {} {}",
        repeated_buffer(129, 0),
        repeated_buffer(128, 129)
    );
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("copy with one buffer", b"0 1 1 7".to_vec()),
        ("merge with one buffer", b"2 1 1 7".to_vec()),
        ("merge above maximum", merge_overflow.into_bytes()),
        ("missing split position", b"3 1 1 7".to_vec()),
        ("nonnumeric split position", b"3 1 1 7 x".to_vec()),
        ("split beyond end", b"3 1 1 7 2".to_vec()),
        ("negative split position", b"3 1 1 7 -1".to_vec()),
        ("interleave with one buffer", b"4 1 1 7".to_vec()),
        (
            "interleave above maximum",
            interleave_overflow.into_bytes(),
        ),
        ("missing rotation amount", b"5 1 1 7".to_vec()),
        ("nonnumeric rotation amount", b"5 1 1 7 x".to_vec()),
        ("unknown operation", b"7 1 1 7".to_vec()),
        ("negative unknown operation", b"-1 1 0".to_vec()),
    ];

    for (name, input) in cases {
        assert_matches(name, input);
    }
}

#[test]
fn maximum_limits_match() {
    let max_buffer = repeated_buffer(256, -128);
    assert_matches("reverse maximum length", format!("1 1 {max_buffer}"));

    let merge_exact = format!(
        "2 2 {} {}",
        repeated_buffer(128, 0),
        repeated_buffer(128, 128)
    );
    assert_matches("merge at maximum combined length", merge_exact);

    let interleave_exact = format!(
        "4 2 {} {}",
        repeated_buffer(128, 0),
        repeated_buffer(128, 128)
    );
    assert_matches("interleave at maximum combined length", interleave_exact);

    let mut max_count = String::from("6 100");
    for _ in 0..100 {
        max_count.push_str(" 0");
    }
    assert_matches("maximum buffer count", max_count);
}
