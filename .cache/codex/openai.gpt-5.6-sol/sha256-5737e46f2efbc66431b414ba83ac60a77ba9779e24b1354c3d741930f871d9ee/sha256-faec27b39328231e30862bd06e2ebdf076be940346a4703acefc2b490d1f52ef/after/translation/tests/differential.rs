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
        .expect("piped stdin is available")
        .write_all(input)
        .unwrap_or_else(|error| panic!("failed to write stdin for {}: {error}", binary.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {}: {error}", binary.display()))
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
fn matches_c_for_all_input_and_control_flow_classes() {
    let cases: &[(&str, &[u8])] = &[
        ("empty input", b""),
        ("whitespace-only input", b" \t\n\r\x0b\x0c"),
        ("invalid first item", b"not-an-integer 1 4\n"),
        ("sign without digits", b"+"),
        ("single zero item", b"0\n"),
        ("single positive item", b"3\n"),
        ("invalid second item", b"2 invalid\n"),
        ("two items split across lines", b"1\n4\n"),
        ("both operands nonpositive", b"-1 -2\n"),
        ("loop entered through y only", b"0 3\n"),
        ("special x-one y-four jump", b"1 4\n"),
        ("x-less-than-three jump", b"2 3\n"),
        ("x-at-least-three fallthrough", b"4 2\n"),
        ("y-zero continue", b"4 0\n"),
        ("extra item ignored", b"2 1 999\n"),
        ("minimum signed int", b"-2147483648 -2147483648\n"),
        ("positive signed int overflow", b"2147483648 0\n"),
        (
            "positive signed int overflow to negative",
            b"2147483649 0\n",
        ),
        ("negative signed int overflow to zero", b"-4294967296 0\n"),
        (
            "negative signed int overflow to negative",
            b"-4294967297 0\n",
        ),
        ("positive long overflow", b"9223372036854775808 0\n"),
        ("negative long overflow", b"-9223372036854775809 0\n"),
    ];

    for &(case, input) in cases {
        assert_matches_c(case, input);
    }
}

#[test]
fn matches_c_for_bounded_integer_cross_product() {
    for x in -3..=8 {
        for y in -3..=8 {
            // In C, positive x with negative y enters the loop and decrements y forever.
            if x > 0 && y < 0 {
                continue;
            }
            let case = format!("bounded pair x={x}, y={y}");
            let input = format!("{x} {y}\n");
            assert_matches_c(&case, input.as_bytes());
        }
    }
}
