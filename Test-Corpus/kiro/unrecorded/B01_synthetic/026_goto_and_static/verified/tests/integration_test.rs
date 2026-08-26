use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::process::Command;

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

/// Test Rust multi_stage via .so for all code paths.
/// The C multi_stage is static (not exported), so we verify the Rust
/// export matches the C logic by testing all branches.
fn call_rust_multi_stage(x: c_int, y: c_int, z: c_int) -> c_int {
    unsafe {
        let lib = Library::new(rust_so_path()).expect("Failed to load Rust .so");
        let func: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            lib.get(b"multi_stage").expect("Failed to find multi_stage");
        func(x, y, z)
    }
}

/// Run the C executable with given input and capture output.
fn run_c_executable(input: &str) -> (String, i32) {
    let exe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/driver");
    let output = Command::new(exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
            child.wait_with_output()
        })
        .expect("Failed to run C executable");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, code)
}

/// Run the Rust executable with given input and capture output.
fn run_rust_executable(input: &str) -> (String, i32) {
    let exe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/driver");
    let output = Command::new(exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
            child.wait_with_output()
        })
        .expect("Failed to run Rust executable");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, code)
}

#[test]
fn test_multi_stage_all_correct() {
    // x=1, y=2, z=3 -> Ok!, result=0
    assert_eq!(call_rust_multi_stage(1, 2, 3), 0);
}

#[test]
fn test_multi_stage_x_wrong() {
    // x!=1 -> result=1
    assert_eq!(call_rust_multi_stage(0, 2, 3), 1);
    assert_eq!(call_rust_multi_stage(5, 2, 3), 1);
}

#[test]
fn test_multi_stage_y_wrong() {
    // x=1, y!=2 -> result=2
    assert_eq!(call_rust_multi_stage(1, 0, 3), 2);
    assert_eq!(call_rust_multi_stage(1, 99, 3), 2);
}

#[test]
fn test_multi_stage_z_wrong() {
    // x=1, y=2, z!=3 -> result=3
    assert_eq!(call_rust_multi_stage(1, 2, 0), 3);
    assert_eq!(call_rust_multi_stage(1, 2, 99), 3);
}

#[test]
fn test_executable_output_match_ok() {
    let input = "1 2 3\n";
    let (c_out, _) = run_c_executable(input);
    let (r_out, _) = run_rust_executable(input);
    assert_eq!(c_out, r_out, "Output mismatch for input '1 2 3'");
}

#[test]
fn test_executable_output_match_x_wrong() {
    let input = "0 2 3\n";
    let (c_out, _) = run_c_executable(input);
    let (r_out, _) = run_rust_executable(input);
    assert_eq!(c_out, r_out, "Output mismatch for input '0 2 3'");
}

#[test]
fn test_executable_output_match_y_wrong() {
    let input = "1 0 3\n";
    let (c_out, _) = run_c_executable(input);
    let (r_out, _) = run_rust_executable(input);
    assert_eq!(c_out, r_out, "Output mismatch for input '1 0 3'");
}

#[test]
fn test_executable_output_match_z_wrong() {
    let input = "1 2 0\n";
    let (c_out, _) = run_c_executable(input);
    let (r_out, _) = run_rust_executable(input);
    assert_eq!(c_out, r_out, "Output mismatch for input '1 2 0'");
}
