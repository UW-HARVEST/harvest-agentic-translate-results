use libloading::{Library, Symbol};
use std::process::Command;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

/// Capture stdout from the C printHexCharLine by running a small helper via the C main binary.
/// Since printHexCharLine writes to stdout, we compare outputs by running both binaries.

/// Helper: run C executable with given input byte, return stdout
fn run_c_binary(input: &[u8]) -> Vec<u8> {
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");
    let out = Command::new(c_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(input).ok();
            child.wait_with_output()
        })
        .expect("failed to run C binary");
    out.stdout
}

/// Helper: run Rust executable with given input byte, return stdout
fn run_rust_binary(input: &[u8]) -> Vec<u8> {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let out = Command::new(rust_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(input).ok();
            child.wait_with_output()
        })
        .expect("failed to run Rust binary");
    out.stdout
}

#[test]
fn test_printHexCharLine_via_binary_comparison() {
    // Test a range of interesting input bytes
    let test_inputs: Vec<u8> = vec![
        0x00, 0x01, 0x20, 0x41, 0x7e, 0x7f, 0x80, 0xfe, 0xff,
    ];

    for &b in &test_inputs {
        let c_out = run_c_binary(&[b]);
        let r_out = run_rust_binary(&[b]);
        assert_eq!(
            c_out, r_out,
            "Mismatch for input byte 0x{:02x}: C={:?} Rust={:?}",
            b,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}

#[test]
fn test_c_library_loads() {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C shared library");
        // Verify printHexCharLine symbol exists
        let _func: Symbol<unsafe extern "C" fn(i8)> =
            lib.get(b"printHexCharLine").expect("printHexCharLine not found");
    }
}
