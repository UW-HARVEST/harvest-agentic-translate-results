use libloading::{Library, Symbol};
use std::process::Command;
use std::io::Write;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const C_BIN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");

fn run_binary(bin: &str, input: &str) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", bin, e));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap().stdout
}

#[test]
fn test_driver_via_binaries() {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let inputs = ["0", "1", "-1", "255", "256", "2147483647", "-2147483648", "12345678"];
    for input in &inputs {
        let c_out = run_binary(C_BIN_PATH, input);
        let r_out = run_binary(rust_bin, input);
        assert_eq!(c_out, r_out,
            "stdout mismatch for input '{}': C={:?} Rust={:?}",
            input, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}

#[test]
fn test_driver_ffi_symbols() {
    let c_lib = unsafe { Library::new(C_LIB_PATH) }.expect("failed to load C lib");
    let rust_lib_path = format!("{}/target/debug/libdriver.so", env!("CARGO_MANIFEST_DIR"));
    let rust_lib = unsafe { Library::new(&rust_lib_path) }.expect("failed to load Rust lib");
    unsafe {
        let _: Symbol<unsafe extern "C" fn(i32)> = c_lib.get(b"driver").expect("C lib missing driver");
        let _: Symbol<unsafe extern "C" fn(i32)> = rust_lib.get(b"driver").expect("Rust lib missing driver");
    }
}
