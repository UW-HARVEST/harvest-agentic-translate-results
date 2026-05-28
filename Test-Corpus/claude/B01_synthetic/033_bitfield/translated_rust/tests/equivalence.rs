// Equivalence tests: compare C reference binary vs Rust binary on stdin/stdout.
//
// Rationale: this project's C source compiles to an executable (not a shared
// library) that reads four integers from stdin via scanf and writes one line
// to stdout via printf. The Rust translation in src/main.rs is the same shape:
// an executable with the same I/O contract. There are no exported FFI symbols
// to compare with libloading; the public observable interface IS stdin/stdout.
// So we test equivalence by spawning each binary and diffing output.
//
// Cargo.toml has no [features] section, so there is only one configuration.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    project_root().join("c_src/build/driver")
}

fn rust_binary() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn run(bin: &PathBuf, input: &str) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {}", bin, e));
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (out.stdout, out.stderr, out.status.code())
}

fn assert_match(input: &str) {
    let c_bin = c_binary();
    let r_bin = rust_binary();
    assert!(
        c_bin.exists(),
        "C reference binary not built; run cmake build in c_src/build first: {:?}",
        c_bin
    );
    let (c_stdout, _c_stderr, c_code) = run(&c_bin, input);
    let (r_stdout, _r_stderr, r_code) = run(&r_bin, input);
    assert_eq!(
        c_stdout, r_stdout,
        "stdout mismatch for input {:?}\n C: {:?}\n R: {:?}",
        input,
        String::from_utf8_lossy(&c_stdout),
        String::from_utf8_lossy(&r_stdout)
    );
    assert_eq!(
        c_code, r_code,
        "exit code mismatch for input {:?}: C={:?} R={:?}",
        input, c_code, r_code
    );
}

#[test]
fn all_zero() {
    assert_match("0 0 0 0\n");
}

#[test]
fn small_in_range() {
    assert_match("1 2 1 42\n");
}

#[test]
fn negative_z() {
    assert_match("3 7 1 -1\n");
}

#[test]
fn x_overflow_2bit_field() {
    // x=4 wraps to 0 (2-bit field), y=8 wraps to 0 (3-bit field), b=5 -> !!5 = 1
    assert_match("4 8 5 100\n");
}

#[test]
fn x_y_at_field_max() {
    assert_match("7 15 1 2147483647\n");
}

#[test]
fn z_int_min() {
    assert_match("0 0 1 -2147483648\n");
}

#[test]
fn b_falsy() {
    assert_match("5 5 0 -100\n");
}

#[test]
fn larger_values_truncated() {
    assert_match("100 200 1 0\n");
}

#[test]
fn ones() {
    assert_match("1 1 1 1\n");
}

#[test]
fn z_negative_b_zero() {
    assert_match("2 4 0 -42\n");
}

#[test]
fn extra_whitespace_and_signs() {
    // scanf skips leading whitespace and accepts a sign before %u as well.
    assert_match("  3   6   1   -7  \n");
}

#[test]
fn empty_input_keeps_zero_initialized_vars() {
    // C: variables initialized to 0; scanf returns -1 (EOF) but vars stay 0.
    assert_match("");
}

#[test]
fn partial_input_three_tokens() {
    // Only three tokens parse; z stays 0 in both implementations.
    assert_match("1 2 1\n");
}

#[test]
fn many_random_inputs() {
    // Pseudo-random but deterministic sweep.
    let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
    for _ in 0..200 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = (state >> 32) as u32 & 0xFFFF;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let y = (state >> 32) as u32 & 0xFFFF;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = ((state >> 32) as i32) % 7; // sometimes 0, sometimes nonzero of either sign
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let z = (state >> 32) as i32;
        let input = format!("{} {} {} {}\n", x, y, b, z);
        assert_match(&input);
    }
}
