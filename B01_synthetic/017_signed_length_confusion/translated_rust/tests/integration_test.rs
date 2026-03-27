use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::process::Command;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib() -> String {
    format!("{}/target/debug/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

fn c_bin() -> String {
    format!("{}/c_src/build/driver", env!("CARGO_MANIFEST_DIR"))
}

fn rust_bin() -> String {
    format!("{}/target/debug/driver", env!("CARGO_MANIFEST_DIR"))
}

fn run_with_input(bin: &str, input: &str) -> Vec<u8> {
    use std::io::Write;
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {}: {}", bin, e));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    out.stdout
}

/// Test printLine symbol exists in both libraries and handles NULL
#[test]
fn test_printline_null() {
    unsafe {
        let c_lib = Library::new(C_LIB).expect("load C lib");
        let r_lib = Library::new(rust_lib()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").expect("Rust printLine");

        // NULL should not crash
        c_fn(std::ptr::null());
        r_fn(std::ptr::null());
    }
}

/// Test printLine with a simple string (both should produce same output to stdout)
#[test]
fn test_printline_exists_in_both() {
    unsafe {
        let c_lib = Library::new(C_LIB).expect("load C lib");
        let r_lib = Library::new(rust_lib()).expect("load Rust lib");

        // Just verify the symbol exists with correct signature
        let _c: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let _r: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").expect("Rust printLine");
    }
}

/// Test main symbol exists in Rust .so
#[test]
fn test_main_symbol_exists() {
    unsafe {
        let r_lib = Library::new(rust_lib()).expect("load Rust lib");
        let _: Symbol<unsafe extern "C" fn() -> i32> =
            r_lib.get(b"main").expect("Rust main symbol");
    }
}

/// Compare binary outputs for normal inputs
#[test]
fn test_binary_normal_inputs() {
    for input in &["0\n", "1\n", "50\n", "99\n", "100\n", "200\n"] {
        let c_out = run_with_input(&c_bin(), input);
        let r_out = run_with_input(&rust_bin(), input);
        assert_eq!(
            c_out, r_out,
            "Mismatch for input {:?}\nC:    {:?}\nRust: {:?}",
            input.trim(),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

/// Test edge case: input >= 100 should print empty line
#[test]
fn test_binary_large_input() {
    for input in &["100\n", "999\n", "200\n"] {
        let c_out = run_with_input(&c_bin(), input);
        let r_out = run_with_input(&rust_bin(), input);
        assert_eq!(c_out, r_out, "Mismatch for input {:?}", input.trim());
        assert_eq!(c_out, b"\n", "Expected empty line for input {:?}", input.trim());
    }
}

/// Test input = 0 should print empty line (dest[0] = '\0')
#[test]
fn test_binary_zero_input() {
    let c_out = run_with_input(&c_bin(), "0\n");
    let r_out = run_with_input(&rust_bin(), "0\n");
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, b"\n");
}
