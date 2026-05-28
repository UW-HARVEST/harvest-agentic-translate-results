// Integration test: the C source and the Rust translation both build standalone
// executables (no shared library / FFI boundary). To verify byte-identical
// behavior we invoke both binaries and diff their stdout, stderr, and exit code.

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    manifest_dir().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo when running integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(path: &PathBuf, args: &[&str]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let out = Command::new(path)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {}", path, e));
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(args: &[&str]) {
    let c = c_binary();
    let r = rust_binary();
    assert!(
        c.exists(),
        "C binary not found at {:?}; build with cmake first",
        c
    );
    let (c_out, c_err, c_code) = run(&c, args);
    let (r_out, r_err, r_code) = run(&r, args);
    assert_eq!(c_out, r_out, "stdout differs for args {:?}", args);
    assert_eq!(c_err, r_err, "stderr differs for args {:?}", args);
    assert_eq!(c_code, r_code, "exit code differs for args {:?}", args);
}

#[test]
fn no_args_matches() {
    assert_match(&[]);
}

#[test]
fn extra_args_matches() {
    // The C main ignores argc/argv beyond declaration; behavior should be identical.
    assert_match(&["foo", "bar", "baz"]);
}

#[test]
fn single_arg_matches() {
    assert_match(&["hello"]);
}
