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
        .expect("failed to write child stdin");

    child.wait_with_output().expect("failed to wait for child")
}

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn assert_matches_c(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(rust.stdout, c.stdout, "{case}: stdout differs");
    assert_eq!(rust.stderr, c.stderr, "{case}: stderr differs");
    assert_eq!(rust.status, c.status, "{case}: exit status differs");
}

#[test]
fn scanf_and_integer_boundaries_match_c() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace-only input", b" \t\r\n"),
        ("non-numeric input", b"not-a-number\n"),
        ("sign without digits", b"+\n"),
        ("zero", b"0\n"),
        ("single item", b"7\n"),
        ("leading whitespace across lines", b"\n\t 42\n"),
        ("only first item is read", b"1 999\n"),
        ("numeric prefix before invalid suffix", b"12xyz\n"),
        ("explicit positive sign", b"+17\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("one above maximum int", b"2147483648\n"),
        ("one below minimum int", b"-2147483649\n"),
        (
            "positive scanf overflow",
            b"999999999999999999999999999999\n",
        ),
        (
            "negative scanf overflow",
            b"-999999999999999999999999999999\n",
        ),
        ("last input before add overflow", b"1073741673\n"),
        ("first positive add overflow", b"1073741674\n"),
        ("largest input before multiply overflow", b"1073741823\n"),
        ("first positive multiply overflow", b"1073741824\n"),
        ("smallest input before multiply overflow", b"-1073741824\n"),
        ("first negative multiply overflow", b"-1073741825\n"),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}
