use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const COMMON_ARGV0: &str = "driver";

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/driver")
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn spawn(binary: &Path, arguments: &[&str]) -> Child {
    let mut command = Command::new(binary);
    command
        .arg0(COMMON_ARGV0)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()))
}

fn assert_matches_c(case_name: &str, arguments: &[&str]) {
    let c_process = spawn(&c_binary(), arguments);
    let rust_process = spawn(rust_binary(), arguments);
    let expected = c_process
        .wait_with_output()
        .expect("failed to collect C output");
    let actual = rust_process
        .wait_with_output()
        .expect("failed to collect Rust output");

    assert_eq!(
        actual.stdout, expected.stdout,
        "{case_name}: stdout differs"
    );
    assert_eq!(
        actual.stderr, expected.stderr,
        "{case_name}: stderr differs"
    );
    assert_eq!(
        actual.status, expected.status,
        "{case_name}: exit status differs"
    );
}

#[test]
fn validation_paths_match() {
    let cases = [
        ("empty command line", &[][..]),
        ("too many arguments", &["1", "2"][..]),
        ("no digits", &["x"][..]),
        ("trailing characters", &["1x"][..]),
        (
            "unsigned long overflow",
            &["18446744073709551616"][..],
        ),
        ("above unsigned int maximum", &["4294967296"][..]),
        ("negative value wrapping above unsigned int", &["-1"][..]),
    ];

    for (case_name, arguments) in cases {
        assert_matches_c(case_name, arguments);
    }
}

#[test]
fn maximum_seed_success_path_matches() {
    assert_matches_c("maximum unsigned int seed", &["4294967295"]);
}
