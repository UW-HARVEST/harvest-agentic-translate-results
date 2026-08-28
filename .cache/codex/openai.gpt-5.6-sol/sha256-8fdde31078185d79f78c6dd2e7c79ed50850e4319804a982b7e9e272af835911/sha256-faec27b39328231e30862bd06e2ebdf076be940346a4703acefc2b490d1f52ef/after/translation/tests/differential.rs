use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn c_driver() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn run(program: &Path, arguments: &[&str]) -> Output {
    Command::new(program)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()))
}

fn assert_matches_c(case: &str, arguments: &[&str]) {
    let c = run(&c_driver(), arguments);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), arguments);

    assert_eq!(
        rust.status, c.status,
        "{case}: exit status differs for arguments {arguments:?}"
    );
    assert_eq!(
        rust.stdout, c.stdout,
        "{case}: stdout differs for arguments {arguments:?}"
    );
    assert_eq!(
        rust.stderr, c.stderr,
        "{case}: stderr differs for arguments {arguments:?}"
    );
}

#[test]
fn argument_count_errors_match() {
    let cases: &[(&str, &[&str])] = &[
        ("empty argument list", &[]),
        ("single argument", &["1"]),
        ("too many arguments", &["1", "1", "extra"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn integer_validation_and_order_match() {
    let cases: &[(&str, &[&str])] = &[
        ("empty first argument", &["", "1"]),
        ("nonnumeric first argument", &["not-a-number", "1"]),
        ("whitespace-only first argument", &[" \t", "1"]),
        ("empty second argument", &["1", ""]),
        ("nonnumeric second argument", &["1", "not-a-number"]),
        ("first error takes precedence", &["bad-first", "bad-second"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn c_style_partial_integer_parsing_matches() {
    let cases: &[(&str, &[&str])] = &[
        ("leading whitespace and sign", &[" \t+2", "2"]),
        ("suffix on first integer", &["2suffix", "2"]),
        ("suffix on second integer", &["2", "2suffix"]),
        ("second integer begins with digits", &["2", "2 99"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn iteration_boundaries_match() {
    let cases: &[(&str, &[&str])] = &[
        ("negative iterations", &["7", "-1"]),
        ("zero iterations", &["7", "0"]),
        ("single iteration", &["7", "1"]),
        ("multiple iterations", &["7", "4"]),
        ("converted iteration becomes negative", &["7", "2147483648"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn static_alias_branches_match() {
    let cases: &[(&str, &[&str])] = &[
        ("outer starts above inner", &["2", "4"]),
        ("outer starts equal to inner", &["1", "4"]),
        ("outer starts below inner", &["0", "4"]),
        ("several outer updates before aliasing", &["-3", "7"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn conversion_and_arithmetic_boundaries_match() {
    let cases: &[(&str, &[&str])] = &[
        ("maximum int", &["2147483647", "1"]),
        ("minimum int", &["-2147483648", "2"]),
        ("maximum long converted to int", &["9223372036854775807", "1"]),
        ("minimum long converted to int", &["-9223372036854775808", "2"]),
        ("positive strtol overflow", &["9223372036854775808", "1"]),
        ("negative strtol overflow", &["-9223372036854775809", "2"]),
        ("aliased doubling crosses int boundary", &["1", "33"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}
