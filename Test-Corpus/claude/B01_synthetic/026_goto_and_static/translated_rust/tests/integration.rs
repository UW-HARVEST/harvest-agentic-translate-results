// Integration tests that run both the C and Rust `driver` binaries on the
// same stdin and compare stdout/stderr/exit-status byte-for-byte.
//
// The C source compiles to an executable (see c_src/CMakeLists.txt). There is
// no shared library and no `#[no_mangle]` FFI surface to load — the public
// "API" of this program is its standard streams. So we drive both binaries as
// subprocesses and assert their externally-observable behavior matches.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    project_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // cargo sets CARGO_BIN_EXE_<name> for [[bin]] targets when building tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn ensure_c_built() {
    let bin = c_binary();
    if bin.exists() {
        return;
    }
    let c_src = project_root().join("c_src");
    let build_dir = c_src.join("build");
    std::fs::create_dir_all(&build_dir).expect("create build dir");
    let status = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build_dir)
        .status()
        .expect("run cmake configure");
    assert!(status.success(), "cmake configure failed");
    let status = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .current_dir(&build_dir)
        .status()
        .expect("run cmake build");
    assert!(status.success(), "cmake build failed");
}

#[derive(Debug, PartialEq, Eq)]
struct RunOutput {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run(bin: &Path, stdin_bytes: &[u8]) -> RunOutput {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {:?}: {}", bin, e));

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(stdin_bytes).expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait_with_output");
    RunOutput {
        status: out.status.code(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn compare(label: &str, stdin_bytes: &[u8]) {
    ensure_c_built();
    let c = run(&c_binary(), stdin_bytes);
    let r = run(&rust_binary(), stdin_bytes);
    assert_eq!(
        c.stdout, r.stdout,
        "[{label}] stdout mismatch\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr, r.stderr,
        "[{label}] stderr mismatch\nC: {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(c.status, r.status, "[{label}] exit status mismatch");
}

#[test]
fn happy_path_1_2_3() {
    compare("happy_path", b"1 2 3\n");
}

#[test]
fn x_not_one() {
    compare("x_not_one", b"0 2 3\n");
}

#[test]
fn y_not_two() {
    compare("y_not_two", b"1 0 3\n");
}

#[test]
fn z_not_three() {
    compare("z_not_three", b"1 2 0\n");
}

#[test]
fn empty_input() {
    // No tokens -> x=0, y stays at 123, z=0 in both languages.
    compare("empty", b"");
}

#[test]
fn whitespace_separated_newlines() {
    compare("whitespace_newlines", b"1\n2\n3\n");
}

#[test]
fn whitespace_separated_tabs() {
    compare("whitespace_tabs", b"1\t2\t3\n");
}

#[test]
fn extra_whitespace() {
    compare("extra_whitespace", b"   1   2   3   \n");
}

#[test]
fn extra_tokens_are_ignored() {
    // scanf only reads three; any trailing input should be ignored.
    compare("extra_tokens", b"1 2 3 4 5 6\n");
}

#[test]
fn negative_x() {
    compare("negative_x", b"-1 2 3\n");
}

#[test]
fn negative_y() {
    compare("negative_y", b"1 -2 3\n");
}

#[test]
fn negative_z() {
    compare("negative_z", b"1 2 -3\n");
}

#[test]
fn explicit_plus_signs() {
    compare("plus_signs", b"+1 +2 +3\n");
}

#[test]
fn only_one_token() {
    // x=5, y stays default, z=0
    compare("only_one_token", b"5\n");
}

#[test]
fn only_two_tokens_y_set() {
    // y=2 from input, z stays 0
    compare("only_two_tokens", b"1 2\n");
}

#[test]
fn x_one_y_two_z_two() {
    compare("x1_y2_z2", b"1 2 2\n");
}

#[test]
fn large_values() {
    compare("large_values", b"2147483647 -2147483648 0\n");
}

#[test]
fn x_one_y_default_no_z() {
    // y stays at 123 (default), so error on y != 2
    compare("y_default", b"1\n");
}
