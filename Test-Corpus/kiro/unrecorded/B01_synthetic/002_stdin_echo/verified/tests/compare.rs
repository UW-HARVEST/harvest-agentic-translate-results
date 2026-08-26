use libloading::{Library, Symbol};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libecho.so")
}

/// Run the test_runner helper binary which loads a .so and calls its main(),
/// piping input and capturing output.
fn call_main_with_input(lib_path: &std::path::Path, input: &[u8]) -> (Vec<u8>, i32) {
    let runner = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/test_runner");
    let mut child = Command::new(&runner)
        .arg(lib_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn test_runner");

    child.stdin.take().unwrap().write_all(input).ok();
    let output = child.wait_with_output().expect("wait failed");
    (output.stdout, output.status.code().unwrap_or(-1))
}

fn compare(input: &[u8]) {
    let (c_out, c_rc) = call_main_with_input(&c_lib_path(), input);
    let (r_out, r_rc) = call_main_with_input(&rust_lib_path(), input);
    assert_eq!(c_rc, r_rc, "exit codes differ for input {:?}", input);
    assert_eq!(c_out, r_out,
        "output differs for input {:?}\nC:    {:?}\nRust: {:?}",
        input, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_empty_input() {
    compare(b"");
}

#[test]
fn test_single_line() {
    compare(b"hello world\n");
}

#[test]
fn test_multiple_lines() {
    compare(b"line1\nline2\nline3\n");
}

#[test]
fn test_no_trailing_newline() {
    compare(b"no newline");
}

#[test]
fn test_long_line_exactly_127() {
    let line: Vec<u8> = std::iter::repeat(b'A').take(127).collect();
    compare(&line);
}

#[test]
fn test_long_line_over_127() {
    let mut line: Vec<u8> = std::iter::repeat(b'B').take(200).collect();
    line.push(b'\n');
    compare(&line);
}

#[test]
fn test_binary_data() {
    let input: Vec<u8> = (0..=255).collect();
    compare(&input);
}

#[test]
fn test_only_newlines() {
    compare(b"\n\n\n");
}

#[test]
fn test_mixed_short_long() {
    let mut input = Vec::new();
    input.extend_from_slice(b"short\n");
    input.extend(std::iter::repeat(b'X').take(300));
    input.push(b'\n');
    input.extend_from_slice(b"end\n");
    compare(&input);
}

/// Verify both .so files export the same symbols
#[test]
fn test_symbol_exports() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        // Both must export "main"
        let _: Symbol<unsafe extern "C" fn() -> i32> = c_lib.get(b"main").expect("C main");
        let _: Symbol<unsafe extern "C" fn() -> i32> = r_lib.get(b"main").expect("Rust main");
    }
}
