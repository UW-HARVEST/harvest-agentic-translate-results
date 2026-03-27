use std::process::Command;
use std::io::Write;

fn run_binary(bin: &str, input: &str) -> Vec<u8> {
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run binary");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap().stdout
}

#[test]
fn test_driver_binary_output_matches() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest);
    let rust_bin = format!("{}/target/debug/driver", manifest);

    // Build Rust binary
    let out = Command::new("cargo")
        .args(["build", "--bin", "driver"])
        .current_dir(manifest)
        .output()
        .unwrap();
    assert!(out.status.success(), "cargo build: {}", String::from_utf8_lossy(&out.stderr));

    for input in &["0", "1", "5", "100", "-1", "2147483647"] {
        let c_out = run_binary(&c_bin, input);
        let r_out = run_binary(&rust_bin, input);
        assert_eq!(
            c_out, r_out,
            "Mismatch for input '{}'\nC:    {}\nRust: {}",
            input,
            String::from_utf8_lossy(&c_out).trim(),
            String::from_utf8_lossy(&r_out).trim()
        );
    }
}

#[test]
fn test_driver_symbol_loadable() {
    let manifest = env!("CARGO_MANIFEST_DIR");

    // Build Rust cdylib
    let out = Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(manifest)
        .output()
        .unwrap();
    assert!(out.status.success(), "cargo build --lib: {}", String::from_utf8_lossy(&out.stderr));

    let c_lib_path = format!("{}/c_src/build/libdriver.so", manifest);
    let rust_lib_path = format!("{}/target/debug/libdriver.so", manifest);

    unsafe {
        let c_lib = libloading::Library::new(&c_lib_path).expect("Failed to load C .so");
        let _c_driver: libloading::Symbol<unsafe extern "C" fn(i32)> =
            c_lib.get(b"driver").expect("C .so missing 'driver'");

        let r_lib = libloading::Library::new(&rust_lib_path).expect("Failed to load Rust .so");
        let _r_driver: libloading::Symbol<unsafe extern "C" fn(i32)> =
            r_lib.get(b"driver").expect("Rust .so missing 'driver'");
    }
}
