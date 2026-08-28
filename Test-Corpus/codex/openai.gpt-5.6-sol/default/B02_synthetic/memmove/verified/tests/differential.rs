use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Case {
    name: String,
    input: Vec<u8>,
}

impl Case {
    fn new(name: impl Into<String>, input: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            input: input.into(),
        }
    }
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(path: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", path.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", path.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", path.display()))
}

fn compare(case: Case) {
    let c = run(&c_driver(), &case.input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), &case.input);

    assert_eq!(
        rust.stdout, c.stdout,
        "{}: stdout differs\ninput: {:?}\nC: {:?}\nRust: {:?}",
        case.name, case.input, c.stdout, rust.stdout
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{}: stderr differs\ninput: {:?}\nC: {:?}\nRust: {:?}",
        case.name, case.input, c.stderr, rust.stderr
    );
    assert_eq!(
        rust.status, c.status,
        "{}: exit status differs\ninput: {:?}\nC: {:?}\nRust: {:?}",
        case.name, case.input, c.status, rust.status
    );
}

#[test]
fn input_and_error_paths_match() {
    let cases = [
        Case::new("empty input", b"".as_slice()),
        Case::new("invalid flags", b"x".as_slice()),
        Case::new("missing param1", b"0".as_slice()),
        Case::new("invalid param1", b"0 x".as_slice()),
        Case::new("missing param2", b"0 0".as_slice()),
        Case::new("invalid param2", b"0 0 x".as_slice()),
        Case::new("missing length", b"0 0 0".as_slice()),
        Case::new("invalid length", b"0 0 0 x".as_slice()),
        Case::new("length above maximum", b"0 0 0 257".as_slice()),
        Case::new("negative length wraps", b"0 0 0 -1".as_slice()),
        Case::new("missing first byte", b"0 0 0 1".as_slice()),
        Case::new("invalid first byte", b"0 0 0 1 x".as_slice()),
        Case::new("missing later byte", b"0 0 0 3 10 20".as_slice()),
        Case::new("invalid later byte", b"0 0 0 3 10 20 x".as_slice()),
        Case::new(
            "missing final byte at maximum length",
            format!("0 0 0 256 {}", "7 ".repeat(255)).into_bytes(),
        ),
        Case::new(
            "scanf whitespace crosses lines",
            b"\n0\t0\r\n0\x0b3\x0c1\n2 3 trailing text".as_slice(),
        ),
        Case::new("sign without digits", b"+".as_slice()),
        Case::new("nul is not whitespace", b"\0".as_slice()),
    ];

    for case in cases {
        compare(case);
    }
}

#[test]
fn operation_branches_match() {
    let cases = [
        Case::new("zero length early return", b"31 -5 1 0".as_slice()),
        Case::new("single byte all flags", b"31 7 1 1 300".as_slice()),
        Case::new("no operation flags", b"0 9 0 5 0 1 127 255 256".as_slice()),
        Case::new(
            "unknown flags ignored",
            b"4294967264 0 0 4 1 2 3 4".as_slice(),
        ),
        Case::new("rotate zero offset", b"1 12 0 6 1 2 3 4 5 6".as_slice()),
        Case::new(
            "rotate small positive offset",
            b"1 2 0 6 1 2 3 4 5 6".as_slice(),
        ),
        Case::new(
            "rotate large positive offset",
            b"1 3 0 6 1 2 3 4 5 6".as_slice(),
        ),
        Case::new("rotate negative offset", b"1 -2 0 6 1 2 3 4 5 6".as_slice()),
        Case::new(
            "compact default threshold with movement",
            b"2 0 0 10 4 4 4 5 6 6 6 6 7 7".as_slice(),
        ),
        Case::new(
            "compact threshold above 255 defaults",
            b"2 256 0 7 8 8 8 9 9 9 9".as_slice(),
        ),
        Case::new("compact threshold one", b"2 1 0 5 1 2 2 3 4".as_slice()),
        Case::new(
            "compact keeps short runs",
            b"2 3 0 6 1 1 2 2 3 3".as_slice(),
        ),
        Case::new(
            "deduplicate preserving order",
            b"4 0 1 9 3 1 3 2 1 4 2 5 4".as_slice(),
        ),
        Case::new(
            "deduplicate with swaps",
            b"4 0 0 9 3 1 3 2 1 4 2 5 4".as_slice(),
        ),
        Case::new("deduplicate single byte", b"4 0 0 1 9".as_slice()),
        Case::new("interleave even length", b"8 0 0 6 1 2 3 4 5 6".as_slice()),
        Case::new("interleave odd length", b"8 0 0 7 1 2 3 4 5 6 7".as_slice()),
        Case::new(
            "interleave guard after deduplication",
            b"12 0 1 4 9 9 9 9".as_slice(),
        ),
        Case::new(
            "reverse default segments and remainder",
            b"16 0 0 10 1 2 3 4 5 6 7 8 9 10".as_slice(),
        ),
        Case::new("reverse segment size one", b"16 1 0 5 1 2 3 4 5".as_slice()),
        Case::new(
            "reverse segment too large",
            b"16 6 0 5 1 2 3 4 5".as_slice(),
        ),
        Case::new("reverse exact segments", b"16 3 0 6 1 2 3 4 5 6".as_slice()),
        Case::new(
            "reverse one-byte remainder",
            b"16 3 0 7 1 2 3 4 5 6 7".as_slice(),
        ),
        Case::new(
            "reverse multi-byte remainder",
            b"16 4 0 11 1 2 3 4 5 6 7 8 9 10 11".as_slice(),
        ),
        Case::new(
            "reverse guard after compaction",
            b"18 3 0 5 8 8 8 8 8".as_slice(),
        ),
        Case::new(
            "all operations chained",
            b"31 3 1 12 1 1 1 2 3 3 3 4 2 5 5 5".as_slice(),
        ),
    ];

    for case in cases {
        compare(case);
    }
}

#[test]
fn numeric_conversion_boundaries_match() {
    let cases = [
        Case::new("negative unsigned flags", b"-1 0 0 2 1 2".as_slice()),
        Case::new(
            "flags truncate at 32 bits",
            b"4294967296 0 0 1 7".as_slice(),
        ),
        Case::new(
            "flags overflow unsigned long",
            b"18446744073709551616 0 0 1 7".as_slice(),
        ),
        Case::new(
            "positive signed int truncation",
            b"1 2147483648 0 3 1 2 3".as_slice(),
        ),
        Case::new(
            "negative signed int truncation",
            b"1 -2147483649 0 3 1 2 3".as_slice(),
        ),
        Case::new(
            "positive signed long overflow",
            b"1 18446744073709551616 0 3 1 2 3".as_slice(),
        ),
        Case::new(
            "negative signed long overflow",
            b"1 -18446744073709551616 0 3 1 2 3".as_slice(),
        ),
        Case::new(
            "byte truncates to uint8",
            b"0 0 0 4 -1 256 257 4294967295".as_slice(),
        ),
        Case::new(
            "byte overflow unsigned long",
            b"0 0 0 1 18446744073709551616".as_slice(),
        ),
    ];

    for case in cases {
        compare(case);
    }
}

#[test]
fn maximum_length_and_run_match() {
    let mut input = String::from("2 255 0 256");
    for _ in 0..256 {
        input.push_str(" 42");
    }
    input.push('\n');

    compare(Case::new(
        "maximum length run capped at 255",
        input.into_bytes(),
    ));
}

#[test]
fn all_flag_combinations_match() {
    let scenarios = [
        (-3, 0, Vec::new()),
        (7, 1, vec![9]),
        (1, 0, vec![1, 2]),
        (2, 1, vec![1, 1, 2]),
        (3, 0, vec![1, 1, 1, 2]),
        (4, 1, vec![3, 1, 3, 2, 1]),
        (-1, 0, vec![1, 2, 2, 3, 3, 3, 4]),
        (5, 1, vec![1, 1, 1, 2, 3, 3, 3, 4, 5]),
        (3, 0, (0..=255).map(|value| value as u8).collect()),
    ];

    for flags in 0..32_u32 {
        for (param1, param2, buffer) in &scenarios {
            let mut input = format!("{flags} {param1} {param2} {}", buffer.len());
            for byte in buffer {
                input.push_str(&format!(" {byte}"));
            }

            compare(Case::new(
                format!("flags {flags:#04x}, length {}", buffer.len()),
                input.into_bytes(),
            ));
        }
    }
}
