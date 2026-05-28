// Integration tests comparing the C and Rust binaries.
//
// This project does not build a shared library: c_src/CMakeLists.txt
// builds an executable, and Cargo.toml declares only a [[bin]]. There
// are no [features] in Cargo.toml, so the single valid configuration
// is the default one. These tests therefore compare program behaviour
// at the executable boundary, which is the actual public interface of
// the program.
//
// `libloading` is still added to [dev-dependencies] per the task
// instructions even though there are no .so files to load here.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_binary_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("driver");
    p
}

fn rust_binary_path() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Run a binary, feeding `input` to its stdin, and return (stdout, stderr,
/// exit_status_code).
fn run_with_stdin(bin: &PathBuf, input: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn binary");

    if !input.is_empty() {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(input)
            .expect("write stdin");
    }
    // Drop stdin to send EOF.
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait");
    (output.stdout, output.stderr, output.status.code())
}

fn assert_match(input: &[u8], label: &str) {
    let c_bin = c_binary_path();
    let rs_bin = rust_binary_path();
    assert!(
        c_bin.exists(),
        "C binary missing at {:?} — build c_src first",
        c_bin
    );
    assert!(rs_bin.exists(), "Rust binary missing at {:?}", rs_bin);

    let (c_out, _c_err, c_code) = run_with_stdin(&c_bin, input);
    let (r_out, _r_err, r_code) = run_with_stdin(&rs_bin, input);

    assert_eq!(
        c_out, r_out,
        "stdout mismatch for case {}: C={:?} Rust={:?}",
        label, c_out, r_out
    );
    assert_eq!(
        c_code, r_code,
        "exit code mismatch for case {}: C={:?} Rust={:?}",
        label, c_code, r_code
    );
}

#[test]
fn matches_on_empty_stdin() {
    // fscanf returns EOF without storing — `data` stays as ' ' (0x20).
    // result = ' ' + 1 = '!' (0x21). Output: "21\n".
    assert_match(b"", "empty");
}

#[test]
fn matches_on_space() {
    assert_match(b" ", "space");
}

#[test]
fn matches_on_zero_byte() {
    assert_match(b"\0", "nul");
}

#[test]
fn matches_on_newline() {
    assert_match(b"\n", "newline");
}

#[test]
fn matches_on_letter() {
    assert_match(b"A", "letter_A");
}

#[test]
fn matches_on_high_ascii() {
    // 0x7F → result = 0x80, which is negative as signed char,
    // sign-extended to int 0xFFFFFF80 and printed as %x → "ffffff80".
    assert_match(&[0x7Fu8], "0x7F");
}

#[test]
fn matches_on_0xff() {
    // 0xFF → wraps to 0x00 after +1, output "00\n".
    assert_match(&[0xFFu8], "0xFF");
}

#[test]
fn matches_on_0x80() {
    // 0x80 (signed -128). +1 → -127 = 0xFFFFFF81 sign-extended.
    assert_match(&[0x80u8], "0x80");
}

#[test]
fn matches_all_byte_values() {
    // Exhaustive: every possible single-byte input.
    for b in 0u16..=255u16 {
        assert_match(&[b as u8], &format!("byte_0x{:02x}", b));
    }
}

#[test]
fn matches_with_extra_input_after_first_byte() {
    // fscanf %c reads exactly one char; extra trailing input is ignored.
    assert_match(b"X extra trailing data\n", "trailing");
}
