use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));

    std::io::Write::write_all(
        child.stdin.as_mut().expect("child stdin must be piped"),
        input,
    )
    .unwrap_or_else(|error| panic!("failed to write to {}: {error}", program.display()));

    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to collect {}: {error}", program.display()))
}

fn assert_status_eq(case: &str, c_status: ExitStatus, rust_status: ExitStatus) {
    assert_eq!(
        rust_status, c_status,
        "{case}: exit status differs (C: {c_status:?}, Rust: {rust_status:?})"
    );
}

fn assert_matches(case: &str, input: &[u8]) {
    let c_program = c_driver();
    assert!(
        c_program.is_file(),
        "C executable is missing at {}; build it before running these tests",
        c_program.display()
    );

    let c = run(&c_program, input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust.stdout, c.stdout,
        "{case}: stdout differs for input {input:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case}: stderr differs for input {input:?}"
    );
    assert_status_eq(case, c.status, rust.status);
}

macro_rules! differential_case {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            assert_matches(stringify!($name), $input);
        }
    };
}

// scanf returns EOF before assigning x.
differential_case!(empty_input, b"");
differential_case!(whitespace_then_eof, b" \t\r\n");

// scanf returns a matching failure before assigning x.
differential_case!(invalid_token, b"not-a-float\n");
differential_case!(invalid_decimal_point, b".\n");
differential_case!(embedded_nul_before_token, b"\0 1.0\n");

// scanf successfully assigns its single requested item.
differential_case!(single_zero, b"0\n");
differential_case!(single_item, b"1.5\n");
differential_case!(negative_value, b"-12.75\n");
differential_case!(leading_plus_and_whitespace, b" \t+2.25\n");
differential_case!(signed_negative_zero, b"-0\n");
differential_case!(hexadecimal_float, b"0x1.8p+2\n");
differential_case!(maximum_finite_float, b"3.4028234663852886e+38\n");
differential_case!(minimum_positive_subnormal, b"1.401298464324817e-45\n");
differential_case!(rounds_to_nearest_float, b"1.000000059604644775390625\n");
differential_case!(positive_infinity, b"inf\n");
differential_case!(negative_infinity, b"-INFINITY\n");
differential_case!(not_a_number, b"nan\n");

// Successful conversions that also exercise range handling.
differential_case!(positive_overflow, b"1e1000\n");
differential_case!(negative_overflow, b"-1e1000\n");
differential_case!(positive_underflow, b"1e-1000\n");
differential_case!(negative_underflow, b"-1e-1000\n");

// %f skips whitespace across lines, accepts a valid prefix, and reads one item.
differential_case!(value_after_multiple_lines, b"\n\n\t\n4.5\n");
differential_case!(valid_prefix_before_junk, b"6.25junk\n");
differential_case!(only_first_of_multiple_items, b"7.5 999.0\n");
differential_case!(maximum_item_count_with_trailing_data, b"8.5\n9.5\n");
