use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate should have a parent directory")
        .join("c_src/build/driver")
}

fn run(binary: &Path, arguments: &[&str]) -> Output {
    let mut command = Command::new(binary);

    #[cfg(unix)]
    command.arg0(OsStr::new("driver"));

    command
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_matches_c(case: &str, arguments: &[&str]) {
    let c = run(&c_binary(), arguments);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), arguments);

    assert_eq!(
        rust.stdout,
        c.stdout,
        "{case}: stdout differs\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout),
    );
    assert_eq!(
        rust.stderr,
        c.stderr,
        "{case}: stderr differs\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr),
    );
    assert_eq!(
        rust.status, c.status,
        "{case}: exit status differs (C: {}, Rust: {})",
        c.status, rust.status,
    );
}

#[test]
fn rejects_every_wrong_argument_count_class() {
    let cases: &[(&str, &[&str])] = &[
        ("empty input", &[]),
        ("single item", &["1"]),
        ("two items", &["1", "2"]),
        ("more than the maximum", &["1", "2", "3", "4"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn matches_regular_numeric_inputs() {
    let cases: &[(&str, &[&str])] = &[
        ("maximum accepted item count", &["1", "0", "0"]),
        ("three-four-five triangle", &["3", "4", "0"]),
        ("mixed signs and decimals", &["-2.5", "0.125", "9.75"]),
        ("all negative", &["-1", "-2", "-3"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn matches_zero_and_atof_parsing_edges() {
    let cases: &[(&str, &[&str])] = &[
        ("zero vector", &["0", "0", "0"]),
        ("signed zero", &["-0", "+0", "0.0"]),
        ("empty items", &["", "", ""]),
        ("nonnumeric items", &["abc", "xyz", "_"]),
        ("numeric prefixes", &["1junk", "-2tail", "3.5rest"]),
        ("leading whitespace", &["  1", "\t-2", "\n3"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}

#[test]
fn matches_extreme_floating_point_inputs() {
    let cases: &[(&str, &[&str])] = &[
        (
            "maximum finite f32 values",
            &["3.402823466e38", "3.402823466e38", "3.402823466e38"],
        ),
        ("float overflow", &["1e100", "-1e100", "1"]),
        ("float underflow", &["1e-100", "-1e-100", "0"]),
        (
            "minimum positive subnormal f32",
            &["1.401298464e-45", "0", "0"],
        ),
        ("infinities", &["inf", "-inf", "infinity"]),
        ("not-a-number", &["nan", "1", "2"]),
    ];

    for (case, arguments) in cases {
        assert_matches_c(case, arguments);
    }
}
