use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug)]
struct RunResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: ExitStatus,
}

fn run_program(path: &OsStr, input: &[u8]) -> RunResult {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", path.to_string_lossy()));

    child
        .stdin
        .as_mut()
        .expect("piped stdin is available")
        .write_all(input)
        .expect("input can be written");

    let output = child.wait_with_output().expect("program can be awaited");
    RunResult {
        stdout: output.stdout,
        stderr: output.stderr,
        status: output.status,
    }
}

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
        .canonicalize()
        .expect("build the C driver with CMake before running the differential tests")
}

#[test]
fn matches_c_for_all_input_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input (scanf EOF)", b""),
        ("whitespace-only input (scanf EOF)", b" \t\r\n"),
        ("nonnumeric input (scanf matching failure)", b"not-a-number\n"),
        ("sign without digits (scanf matching failure)", b"+\n"),
        ("single zero", b"0\n"),
        ("single positive item", b"42\n"),
        ("single negative item", b"-17\n"),
        ("explicit plus sign", b"+23\n"),
        ("leading whitespace across newlines", b"\n\n\t 31\n"),
        ("maximum int", b"2147483647\n"),
        ("minimum int", b"-2147483648\n"),
        ("positive int overflow", b"2147483648\n"),
        ("negative int overflow", b"-2147483649\n"),
        ("very large positive value", b"999999999999999999999999999999\n"),
        ("very large negative value", b"-999999999999999999999999999999\n"),
        ("multiple items (only first is read)", b"7\n99\n"),
        ("numeric prefix with trailing text", b"12xyz\n"),
        ("decimal text truncates at decimal point", b"12.5\n"),
    ];

    let c_driver = c_driver();
    let rust_driver = OsStr::new(env!("CARGO_BIN_EXE_driver"));

    for &(name, input) in cases {
        let expected = run_program(c_driver.as_os_str(), input);
        let actual = run_program(rust_driver, input);

        assert_eq!(actual.stdout, expected.stdout, "{name}: stdout differs");
        assert_eq!(actual.stderr, expected.stderr, "{name}: stderr differs");
        assert_eq!(actual.status, expected.status, "{name}: exit status differs");
    }
}
