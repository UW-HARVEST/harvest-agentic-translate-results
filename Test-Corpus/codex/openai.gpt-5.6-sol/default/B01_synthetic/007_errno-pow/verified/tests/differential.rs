use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(binary: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    command.arg0("driver");

    command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_matches(case: &str, args: &[&str]) {
    let c_path = c_binary();
    assert!(
        c_path.is_file(),
        "C executable is missing at {}; build it before running the tests",
        c_path.display()
    );

    let c = run(&c_path, args);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), args);

    assert_eq!(
        rust.stdout, c.stdout,
        "{case}: stdout differs for arguments {args:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case}: stderr differs for arguments {args:?}"
    );
    assert_eq!(
        rust.status, c.status,
        "{case}: exit status differs for arguments {args:?}"
    );
}

#[test]
fn argument_count_paths_match() {
    assert_matches("empty input", &[]);
    assert_matches("single item", &["2"]);
    assert_matches("more than the supported two items", &["2", "3", "4"]);
}

#[test]
fn successful_calculations_match() {
    assert_matches("two items, the maximum supported count", &["2", "8"]);
    assert_matches("fractional values", &["2.5", "-2"]);
    assert_matches("maximum finite double", &["1.7976931348623157e308", "1"]);
}

#[test]
fn base_conversion_errors_match() {
    assert_matches("invalid base", &["not-a-number", "2"]);
    assert_matches("base conversion overflow", &["1e9999", "2"]);
}

#[test]
fn exponent_conversion_errors_match() {
    assert_matches("invalid exponent", &["2", "not-a-number"]);
    assert_matches("exponent conversion overflow", &["2", "1e9999"]);
}

#[test]
fn pow_errors_match() {
    assert_matches("pow domain error", &["-2", "0.5"]);
    assert_matches("pow overflow", &["1e308", "2"]);
    assert_matches("pow underflow", &["1e-308", "2"]);
    assert_matches("pow pole error", &["0", "-1"]);
}

#[test]
fn conversion_edge_cases_match() {
    assert_matches("empty base is accepted as zero", &["", "2"]);
    assert_matches("empty exponent is accepted as zero", &["2", ""]);
    assert_matches("whitespace-only base is accepted as zero", &["   ", "2"]);
    assert_matches("leading whitespace is accepted", &[" 2", "3"]);
    assert_matches("trailing whitespace is rejected", &["2 ", "3"]);
    assert_matches("partial numeric base is rejected", &["2x", "3"]);
    assert_matches("base conversion underflow", &["1e-9999", "2"]);
    assert_matches("exponent conversion underflow", &["2", "1e-9999"]);
    assert_matches(
        "base range error takes precedence over trailing junk",
        &["1e9999x", "bad"],
    );
    assert_matches(
        "exponent range error takes precedence over trailing junk",
        &["2", "1e9999x"],
    );
    assert_matches(
        "base validation happens before exponent validation",
        &["bad", "1e9999"],
    );
}

#[test]
fn special_results_and_formatting_match() {
    assert_matches("zero to the zeroth power", &["0", "0"]);
    assert_matches("negative base with integral exponent", &["-2", "3"]);
    assert_matches("negative zero result", &["-0", "3"]);
    assert_matches("NaN result", &["nan", "2"]);
    assert_matches("infinite result without a pow range error", &["inf", "1"]);
    assert_matches("two-decimal rounding", &["1.005", "1"]);
}
