use std::ffi::c_int;
use std::process::{Command, Stdio};

fn capture_c_driver(x: c_int, y: c_int) -> String {
    let c_lib_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let script = format!(
        "import ctypes,sys;lib=ctypes.CDLL('{}');lib.driver.argtypes=[ctypes.c_int,ctypes.c_int];lib.driver.restype=None;lib.driver({},{});sys.stdout.flush()",
        c_lib_path.display(), x, y
    );
    let output = Command::new("python3")
        .args(["-c", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("python3 failed");
    assert!(output.status.success(), "C driver python wrapper failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn capture_rust_driver(x: c_int, y: c_int) -> String {
    let rust_lib_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    let script = format!(
        "import ctypes,sys;lib=ctypes.CDLL('{}');lib.driver.argtypes=[ctypes.c_int,ctypes.c_int];lib.driver.restype=None;lib.driver({},{});sys.stdout.flush()",
        rust_lib_path.display(), x, y
    );
    let output = Command::new("python3")
        .args(["-c", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("python3 failed");
    assert!(output.status.success(), "Rust driver python wrapper failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn test_case(x: c_int, y: c_int) {
    let c_out = capture_c_driver(x, y);
    let rust_out = capture_rust_driver(x, y);
    assert_eq!(
        c_out, rust_out,
        "Mismatch for driver({}, {}):\n--- C ---\n{}\n--- Rust ---\n{}",
        x, y, c_out, rust_out
    );
}

#[test] fn test_driver_0_0() { test_case(0, 0); }
#[test] fn test_driver_1_0() { test_case(1, 0); }
#[test] fn test_driver_0_1() { test_case(0, 1); }
#[test] fn test_driver_1_1() { test_case(1, 1); }
#[test] fn test_driver_2_2() { test_case(2, 2); }
#[test] fn test_driver_3_3() { test_case(3, 3); }
#[test] fn test_driver_1_4() { test_case(1, 4); }
#[test] fn test_driver_4_1() { test_case(4, 1); }
#[test] fn test_driver_5_5() { test_case(5, 5); }
#[test] fn test_driver_3_0() { test_case(3, 0); }
#[test] fn test_driver_0_3() { test_case(0, 3); }
#[test] fn test_driver_2_4() { test_case(2, 4); }
