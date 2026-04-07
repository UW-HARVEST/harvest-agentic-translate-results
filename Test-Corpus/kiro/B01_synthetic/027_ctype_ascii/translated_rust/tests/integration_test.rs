use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::process::{Command, Stdio};

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    // Find the Rust .so in target/release
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/target/release");
    format!("{}/libdriver.so", dir)
}

/// Call driver(c) via the given .so, capturing stdout.
fn call_driver_capturing(lib_path: &str, c: c_char) -> String {
    // We need to capture stdout. Since driver() writes to stdout via printf/write,
    // we fork a child process that loads the lib and calls driver.
    // Use a small helper script approach: pipe through a subprocess.
    let output = Command::new("python3")
        .arg("-c")
        .arg(format!(
            r#"
import ctypes, sys, os
lib = ctypes.CDLL("{}")
# Redirect stdout to a pipe
r, w = os.pipe()
old = os.dup(1)
os.dup2(w, 1)
lib.driver(ctypes.c_char({}))
sys.stdout.flush()
os.fsync(1)
os.dup2(old, 1)
os.close(w)
data = b""
while True:
    chunk = os.read(r, 4096)
    if not chunk:
        break
    data += chunk
os.close(r)
sys.stdout.buffer.write(data)
"#,
            lib_path, c as i32
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run python3");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_driver_all_ascii() {
    let c_lib = C_LIB;
    let rust_lib = rust_lib_path();

    // Verify both libs exist
    assert!(
        std::path::Path::new(c_lib).exists(),
        "C library not found at {}",
        c_lib
    );
    assert!(
        std::path::Path::new(&rust_lib).exists(),
        "Rust library not found at {}",
        rust_lib
    );

    let mut mismatches = Vec::new();

    for byte in 0u8..128 {
        let c = byte as i8 as c_char;
        let c_out = call_driver_capturing(c_lib, c);
        let rust_out = call_driver_capturing(&rust_lib, c);

        if c_out != rust_out {
            mismatches.push((byte, c_out.clone(), rust_out.clone()));
            eprintln!(
                "MISMATCH for byte {:#04x} ({:?}):\n  C:    {:?}\n  Rust: {:?}",
                byte, byte as char, c_out, rust_out
            );
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} mismatches found out of 128 test chars",
        mismatches.len()
    );
}

#[test]
fn test_main_export_exists() {
    let rust_lib = rust_lib_path();
    unsafe {
        let lib = Library::new(&rust_lib).expect("failed to load Rust .so");
        let _: Symbol<unsafe extern "C" fn() -> i32> = lib
            .get(b"main")
            .expect("Rust .so must export 'main' symbol");
        let _: Symbol<unsafe extern "C" fn(c_char)> = lib
            .get(b"driver")
            .expect("Rust .so must export 'driver' symbol");
    }
}

#[test]
fn test_nm_symbols_match() {
    let c_lib = C_LIB;
    let rust_lib = rust_lib_path();

    let c_nm = Command::new("nm")
        .args(&["-D", "--defined-only", c_lib])
        .output()
        .expect("nm failed on C lib");
    let rust_nm = Command::new("nm")
        .args(&["-D", "--defined-only", &rust_lib])
        .output()
        .expect("nm failed on Rust lib");

    // Filter out linker-generated symbols (_init, _fini)
    let c_symbols: std::collections::HashSet<String> = String::from_utf8_lossy(&c_nm.stdout)
        .lines()
        .filter(|l| l.contains(" T "))
        .filter_map(|l| l.split_whitespace().last().map(String::from))
        .filter(|s| !s.starts_with('_'))
        .collect();

    let rust_symbols: std::collections::HashSet<String> =
        String::from_utf8_lossy(&rust_nm.stdout)
            .lines()
            .filter(|l| l.contains(" T "))
            .filter_map(|l| l.split_whitespace().last().map(String::from))
            .collect();

    let missing: Vec<_> = c_symbols.difference(&rust_symbols).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing these symbols exported by C .so: {:?}",
        missing
    );
}
