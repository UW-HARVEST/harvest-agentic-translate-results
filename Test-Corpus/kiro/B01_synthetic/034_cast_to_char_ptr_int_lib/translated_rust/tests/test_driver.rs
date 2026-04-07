use libloading::{Library, Symbol};
use std::process::Command;

/// Helper: spawn a process that loads `lib_path`, calls `driver(x)`, captures stdout.
fn call_driver_via_so(lib_path: &str, x: i32) -> String {
    // We use a small inline C-style trick: write a tiny program that dlopen's the lib.
    // But simpler: use a helper script approach. Actually simplest: use LD_PRELOAD trick
    // won't work cleanly. Let's just write a tiny helper binary approach.
    //
    // Simplest reliable approach: use std::process::Command to run a python one-liner
    // that loads the .so via ctypes and calls driver(x).
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes, sys
lib = ctypes.CDLL("{lib_path}")
lib.driver.argtypes = [ctypes.c_int]
lib.driver.restype = None
lib.driver({x})
"#,
        ))
        .output()
        .expect("failed to run python3");
    assert!(output.status.success(), "python3 failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn c_lib_path() -> String {
    std::fs::canonicalize("c_src/build/libdriver.so")
        .expect("C .so not found — build it first")
        .to_str().unwrap().to_string()
}

fn rust_lib_path() -> String {
    std::fs::canonicalize("target/debug/libdriver.so")
        .expect("Rust .so not found — cargo build first")
        .to_str().unwrap().to_string()
}

#[test]
fn test_driver_symbol_exists() {
    unsafe {
        let lib = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        let _: Symbol<unsafe extern "C" fn(i32)> = lib.get(b"driver").expect("missing `driver` symbol");
    }
}

#[test]
fn test_driver_output_matches() {
    let c_path = c_lib_path();
    let rs_path = rust_lib_path();

    for &x in &[0, 1, -1, 42, 255, 256, i32::MAX, i32::MIN, 0x12345678] {
        let c_out = call_driver_via_so(&c_path, x);
        let rs_out = call_driver_via_so(&rs_path, x);
        assert_eq!(c_out, rs_out, "mismatch for driver({x}): C={c_out:?} Rust={rs_out:?}");
    }
}
