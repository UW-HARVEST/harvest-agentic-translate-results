use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

    child.wait_with_output().expect("failed to wait for child")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace then EOF", b" \t\r\n"),
        ("invalid conversion", b"not-a-number\n"),
        ("single item", b"1\n"),
        ("leading whitespace across lines", b"\n\t 12.5\n"),
        ("positive zero", b"0"),
        ("negative zero", b"-0"),
        ("negative finite value", b"-1234.125"),
        ("maximum finite double", b"1.7976931348623157e308"),
        ("minimum positive normal", b"2.2250738585072014e-308"),
        ("minimum positive subnormal", b"4.9406564584124654e-324"),
        ("positive range overflow", b"1e309"),
        ("negative range overflow", b"-1e309"),
        ("positive range underflow", b"1e-4000"),
        ("negative range underflow", b"-1e-4000"),
        ("positive infinity", b"inf"),
        ("negative infinity", b"-infinity"),
        ("not a number", b"nan"),
        ("hexadecimal float", b"0x1.8p+2"),
        ("partial numeric token", b"1e"),
        ("trailing item is ignored", b"3.25\n99.5\n"),
        ("embedded NUL before number", b"\0 7"),
    ];

    let c = c_binary();
    assert!(
        c.is_file(),
        "C executable is missing at {}; build it before running tests",
        c.display()
    );

    let rust = Path::new(env!("CARGO_BIN_EXE_driver"));
    for (name, input) in cases {
        let expected = run(&c, input);
        let actual = run(rust, input);

        assert_eq!(
            actual.stdout, expected.stdout,
            "{name}: stdout differs for input {input:?}"
        );
        assert_eq!(
            actual.stderr, expected.stderr,
            "{name}: stderr differs for input {input:?}"
        );
        assert_eq!(
            actual.status, expected.status,
            "{name}: exit status differs for input {input:?}"
        );
    }
}
