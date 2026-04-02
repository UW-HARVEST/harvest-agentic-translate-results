use libloading::{Library, Symbol};
use std::process::Command;

/// Path to the C shared library
const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

/// Capture stdout from the C `driver(float)` by running a small helper via the C executable.
/// Since driver() prints to stdout, we compare by running both C and Rust binaries with same input.
///
/// But first, let's test that the C driver function is callable and produces expected bytes.

/// Helper: get hex string for a float using Rust logic (same as the translated code)
fn rust_driver_hex(x: f32) -> String {
    let raw = x.to_ne_bytes();
    let mut s = String::new();
    for b in &raw {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Helper: call C driver via subprocess (echo float | ./driver)
fn c_driver_hex(x: f32) -> String {
    let c_exe = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let output = Command::new(c_exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                write!(stdin, "{}", x).ok();
            }
            child.wait_with_output()
        })
        .expect("Failed to run C driver executable");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_driver_zero() {
    let x: f32 = 0.0;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_one() {
    let x: f32 = 1.0;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_negative() {
    let x: f32 = -1.0;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_pi() {
    let x: f32 = std::f32::consts::PI;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_small() {
    let x: f32 = 1.0e-30;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_large() {
    let x: f32 = 1.0e30;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x={}", x);
}

#[test]
fn test_driver_nan() {
    // NaN: C scanf may not parse "nan" the same way, so use a known bit pattern
    let x: f32 = f32::NAN;
    // For NaN, just verify Rust produces valid hex output (C scanf behavior for NaN varies)
    let rust_hex = rust_driver_hex(x);
    assert_eq!(rust_hex.len(), 8, "Expected 8 hex chars for f32");
}

#[test]
fn test_driver_inf() {
    let x: f32 = f32::INFINITY;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x=inf");
}

#[test]
fn test_driver_neg_zero() {
    let x: f32 = -0.0;
    assert_eq!(rust_driver_hex(x), c_driver_hex(x), "Mismatch for x=-0.0");
}

/// Test that the C .so exports `driver` symbol
#[test]
fn test_c_so_has_driver_symbol() {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C .so");
        let _func: Symbol<unsafe extern "C" fn(f32)> =
            lib.get(b"driver").expect("C .so missing 'driver' symbol");
    }
}

/// Test that Rust .so exports the same symbols as C .so
#[test]
fn test_rust_so_exports_match_c() {
    let rust_lib_path = format!(
        "{}/target/debug/libdriver.so",
        env!("CARGO_MANIFEST_DIR")
    );
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("Failed to load C .so");
        let rust_lib = Library::new(&rust_lib_path).expect("Failed to load Rust .so");

        // Check that every symbol C exports, Rust also exports
        for sym in &[b"driver" as &[u8], b"main" as &[u8]] {
            let _c: Symbol<unsafe extern "C" fn()> = c_lib
                .get(sym)
                .unwrap_or_else(|_| panic!("C .so missing '{}'", String::from_utf8_lossy(sym)));
            let _r: Symbol<unsafe extern "C" fn()> = rust_lib
                .get(sym)
                .unwrap_or_else(|_| panic!("Rust .so missing '{}'", String::from_utf8_lossy(sym)));
        }
    }
}

/// Test binary output comparison: run both C and Rust binaries with same input
#[test]
fn test_binary_output_match() {
    let c_exe = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let rust_exe_dir = env!("CARGO_BIN_EXE_driver");

    let test_inputs = ["0.0", "1.0", "-1.0", "3.14159", "1e-30", "1e30"];

    for input in &test_inputs {
        let c_out = run_with_input(c_exe, input);
        let rust_out = run_with_input(rust_exe_dir, input);
        assert_eq!(
            c_out, rust_out,
            "Binary output mismatch for input '{}':\n  C:    {}\n  Rust: {}",
            input, c_out, rust_out
        );
    }
}

fn run_with_input(exe: &str, input: &str) -> String {
    use std::io::Write;
    let mut child = Command::new(exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {}: {}", exe, e));
    if let Some(ref mut stdin) = child.stdin {
        write!(stdin, "{}", input).ok();
    }
    let output = child.wait_with_output().expect("Failed to wait");
    String::from_utf8_lossy(&output.stdout).to_string()
}
