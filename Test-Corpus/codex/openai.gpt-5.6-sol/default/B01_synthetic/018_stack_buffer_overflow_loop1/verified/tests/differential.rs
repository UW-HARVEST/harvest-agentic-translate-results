use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write to {}: {error}", binary.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", binary.display()))
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace-only input", b" \t\n\r"),
        ("invalid conversion", b"abc\n"),
        ("sign without digits", b"-\n"),
        ("single zero item", b"0\n"),
        ("single positive item", b"1\n"),
        ("single negative item", b"-1\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("leading whitespace and plus sign", b" \n\t+42\n"),
        ("integer after multiple newlines", b"\n\n7\n"),
        ("multiple items use the first", b"0 99\n"),
        ("numeric prefix", b"12xyz\n"),
        ("one above maximum int", b"2147483648\n"),
        ("one below minimum int", b"-2147483649\n"),
        (
            "large positive overflow",
            b"999999999999999999999999999999\n",
        ),
        (
            "large negative overflow",
            b"-999999999999999999999999999999\n",
        ),
        ("NUL before digits", b"\0 1\n"),
    ];

    let c = c_binary();
    assert!(
        c.is_file(),
        "C executable is missing; build it first at {}",
        c.display()
    );
    let rust = Path::new(env!("CARGO_BIN_EXE_driver"));

    for &(name, input) in cases {
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
