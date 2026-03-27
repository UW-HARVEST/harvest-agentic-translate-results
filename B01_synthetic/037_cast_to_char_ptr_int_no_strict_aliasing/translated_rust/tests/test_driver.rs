use libloading::{Library, Symbol};
use std::process::Command;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn capture_c_driver(x: i32) -> String {
    // Call C driver via a small helper that forks, redirects stdout, and calls the symbol
    // Simpler: use the C executable directly
    // But we need to test the library function. Use subprocess approach.
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            r#"echo '{}' | LD_LIBRARY_PATH={}/c_src/build {}/c_src/build/driver"#,
            x,
            env!("CARGO_MANIFEST_DIR"),
            env!("CARGO_MANIFEST_DIR")
        ))
        .output()
        .expect("failed to run C driver");
    String::from_utf8(output.stdout).unwrap()
}

fn capture_rust_driver(x: i32) -> String {
    // Build and run the Rust binary
    let bin_path = env!("CARGO_BIN_EXE_driver");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("echo '{}' | {}", x, bin_path))
        .output()
        .expect("failed to run Rust driver");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_driver_zero() {
    let c_out = capture_c_driver(0);
    let r_out = capture_rust_driver(0);
    assert_eq!(c_out, r_out, "driver(0) mismatch");
}

#[test]
fn test_driver_one() {
    let c_out = capture_c_driver(1);
    let r_out = capture_rust_driver(1);
    assert_eq!(c_out, r_out, "driver(1) mismatch");
}

#[test]
fn test_driver_negative() {
    let c_out = capture_c_driver(-1);
    let r_out = capture_rust_driver(-1);
    assert_eq!(c_out, r_out, "driver(-1) mismatch");
}

#[test]
fn test_driver_max() {
    let c_out = capture_c_driver(i32::MAX);
    let r_out = capture_rust_driver(i32::MAX);
    assert_eq!(c_out, r_out, "driver(MAX) mismatch");
}

#[test]
fn test_driver_min() {
    let c_out = capture_c_driver(i32::MIN);
    let r_out = capture_rust_driver(i32::MIN);
    assert_eq!(c_out, r_out, "driver(MIN) mismatch");
}

#[test]
fn test_driver_0xdeadbeef() {
    let x = 0xDEADBEEFu32 as i32;
    let c_out = capture_c_driver(x);
    let r_out = capture_rust_driver(x);
    assert_eq!(c_out, r_out, "driver(0xDEADBEEF) mismatch");
}

#[test]
fn test_driver_via_libloading() {
    // Verify the C .so exports driver and we can call it
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(i32)> = lib.get(b"driver").expect("driver not found");
        // Just verify it doesn't crash - output goes to stdout
        func(42);
    }
}
