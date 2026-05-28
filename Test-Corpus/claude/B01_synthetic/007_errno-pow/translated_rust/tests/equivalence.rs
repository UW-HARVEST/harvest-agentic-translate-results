// Compares the C binary against the Rust binary by running both with the
// same arguments and asserting that stdout, stderr, and exit code match
// byte-for-byte. The C binary is the ground truth.

use std::path::PathBuf;
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // Use the binary built by cargo for the current profile.
    let mut p = workspace_root().join("target");
    // Tests build the package in debug profile by default.
    p.push("debug");
    p.push("driver");
    p
}

fn ensure_binaries_built() {
    let c = c_binary();
    assert!(
        c.exists(),
        "C binary not found at {:?}; build with cmake first",
        c
    );

    let r = rust_binary();
    if !r.exists() {
        // Build the Rust binary so cargo test can locate it.
        let status = Command::new(env!("CARGO"))
            .args(["build", "--bin", "driver"])
            .current_dir(workspace_root())
            .status()
            .expect("cargo build failed to spawn");
        assert!(status.success(), "cargo build failed");
    }
    assert!(
        rust_binary().exists(),
        "Rust binary not found at {:?}",
        rust_binary()
    );
}

fn run(bin: &PathBuf, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e))
}

fn assert_match(args: &[&str]) {
    ensure_binaries_built();
    let c = run(&c_binary(), args);
    let r = run(&rust_binary(), args);

    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code mismatch for args {:?}\nC stdout: {:?}\nC stderr: {:?}\nR stdout: {:?}\nR stderr: {:?}",
        args,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch for args {:?}\nC: {:?}\nR: {:?}",
        args,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    // stderr equality requires the program name in usage messages to match.
    // The C binary uses argv[0] which will be the path to the C executable.
    // The Rust binary uses argv[0] which will be the path to the Rust binary.
    // To compare, we strip the binary name from "Usage: <name> base exponent\n".
    fn normalize_stderr(b: &[u8]) -> Vec<u8> {
        let s = String::from_utf8_lossy(b);
        if let Some(_idx) = s.find("Usage: ") {
            // Replace any "Usage: ... base exponent\n" line with a canonical form.
            let lines: Vec<String> = s
                .lines()
                .map(|l| {
                    if l.starts_with("Usage: ") && l.ends_with(" base exponent") {
                        "Usage: <BIN> base exponent".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            let mut joined = lines.join("\n");
            if s.ends_with('\n') {
                joined.push('\n');
            }
            joined.into_bytes()
        } else {
            b.to_vec()
        }
    }
    let c_err = normalize_stderr(&c.stderr);
    let r_err = normalize_stderr(&r.stderr);
    assert_eq!(
        c_err,
        r_err,
        "stderr mismatch for args {:?}\nC: {:?}\nR: {:?}",
        args,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
}

#[test]
fn small_positive_powers() {
    assert_match(&["2", "3"]);
    assert_match(&["3", "2"]);
    assert_match(&["10", "5"]);
    assert_match(&["1", "1000"]);
}

#[test]
fn fractional_powers() {
    assert_match(&["4", "0.5"]);
    assert_match(&["27", "0.3333333333333333"]);
    assert_match(&["2.5", "2.5"]);
    assert_match(&["1.5", "1.5"]);
}

#[test]
fn negative_base_integer_exponent() {
    assert_match(&["-2", "3"]);
    assert_match(&["-2", "4"]);
    assert_match(&["-1", "2"]);
}

#[test]
fn zero_cases() {
    assert_match(&["0", "0"]);
    assert_match(&["0", "1"]);
    assert_match(&["1", "0"]);
    assert_match(&["0", "5"]);
}

#[test]
fn negative_exponents() {
    assert_match(&["2", "-3"]);
    assert_match(&["10", "-2"]);
    assert_match(&["0.5", "-2"]);
}

#[test]
fn domain_error_negative_base_fractional_exp() {
    // pow(-2, 0.5) is a domain error in IEEE/glibc real-number domain.
    assert_match(&["-2", "0.5"]);
    assert_match(&["-1", "0.25"]);
}

#[test]
fn range_error_overflow() {
    // Should overflow to inf or trigger ERANGE.
    assert_match(&["1e300", "10"]);
    assert_match(&["10", "1000"]);
}

#[test]
fn range_error_underflow() {
    assert_match(&["1e-300", "10"]);
    assert_match(&["0.1", "1000"]);
}

#[test]
fn invalid_base_input() {
    assert_match(&["abc", "2"]);
    assert_match(&["12abc", "2"]);
    assert_match(&["1.2.3", "2"]);
}

#[test]
fn invalid_exponent_input() {
    assert_match(&["2", "abc"]);
    assert_match(&["2", "12abc"]);
    assert_match(&["2", "1.2.3"]);
}

#[test]
fn wrong_arg_count_zero() {
    assert_match(&[]);
}

#[test]
fn wrong_arg_count_one() {
    assert_match(&["2"]);
}

#[test]
fn wrong_arg_count_three() {
    assert_match(&["2", "3", "4"]);
}

#[test]
fn strtod_range_error_for_huge_input() {
    // strtod sets ERANGE if the value is outside the representable range.
    assert_match(&["1e1000", "2"]);
    assert_match(&["2", "1e1000"]);
}

#[test]
fn special_floats_inf_nan_text_form() {
    // glibc's strtod recognizes "inf", "nan", "infinity" (case-insensitive).
    assert_match(&["inf", "2"]);
    assert_match(&["INF", "2"]);
    assert_match(&["nan", "2"]);
    assert_match(&["2", "inf"]);
    assert_match(&["2", "nan"]);
}

#[test]
fn leading_whitespace_in_arg() {
    // strtod skips leading whitespace; both implementations should agree.
    assert_match(&["  2", "3"]);
    assert_match(&["2", "  3"]);
}

#[test]
fn empty_string_arg() {
    assert_match(&["", "3"]);
    assert_match(&["2", ""]);
}

#[test]
fn hex_float_input() {
    // strtod accepts hex float literals like 0x1p10.
    assert_match(&["0x1p10", "2"]);
    assert_match(&["2", "0x1p4"]);
}
