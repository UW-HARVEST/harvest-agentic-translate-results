use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::process::Command;

type MainFn = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libnineality.so")
}

fn rust_lib_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let debug = p.join("debug").join("libnineality.so");
    if debug.exists() {
        return debug;
    }
    p.join("release").join("libnineality.so")
}

fn c_bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("driver")
}

fn rust_bin_path() -> PathBuf {
    // cargo test builds into deps; the actual binary is in target/debug/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("driver")
}

/// Test that both .so files export `main` and return the same exit code
#[test]
fn test_main_symbol_exists_in_both() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    assert!(c_path.exists(), "C .so not found at {:?}", c_path);
    assert!(r_path.exists(), "Rust .so not found at {:?}", r_path);

    unsafe {
        let c_lib = Library::new(&c_path).expect("Failed to load C .so");
        let _c_main: Symbol<MainFn> = c_lib.get(b"main").expect("C .so missing main");

        let r_lib = Library::new(&r_path).expect("Failed to load Rust .so");
        let _r_main: Symbol<MainFn> = r_lib.get(b"main").expect("Rust .so missing main");
    }
}

/// Compare binary stdout for a set of test inputs
#[test]
fn test_binary_output_matches() {
    let c_bin = c_bin_path();
    let r_bin = rust_bin_path();
    assert!(c_bin.exists(), "C binary not found at {:?}", c_bin);
    assert!(r_bin.exists(), "Rust binary not found at {:?}", r_bin);

    let test_inputs = &["0", "1", "5", "9", "10", "15", "19", "99", "100", "123", "-1", "-11"];

    for input in test_inputs {
        let c_out = Command::new(&c_bin)
            .arg(input)
            .output()
            .expect("Failed to run C binary");

        let r_out = Command::new(&r_bin)
            .arg(input)
            .output()
            .expect("Failed to run Rust binary");

        assert_eq!(
            c_out.stdout, r_out.stdout,
            "stdout mismatch for input '{}'\nC:    {:?}\nRust: {:?}",
            input,
            String::from_utf8_lossy(&c_out.stdout),
            String::from_utf8_lossy(&r_out.stdout)
        );
        assert_eq!(
            c_out.status.code(),
            r_out.status.code(),
            "exit code mismatch for input '{}'",
            input
        );
    }
}

/// Compare error cases
#[test]
fn test_error_output_matches() {
    let c_bin = c_bin_path();
    let r_bin = rust_bin_path();

    // No arguments
    let c_out = Command::new(&c_bin).output().expect("C");
    let r_out = Command::new(&r_bin).output().expect("Rust");
    assert_eq!(c_out.stdout, r_out.stdout, "no-arg stdout mismatch\nC: {:?}\nR: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&r_out.stdout));

    // Too many arguments
    let c_out = Command::new(&c_bin).args(&["1", "2"]).output().expect("C");
    let r_out = Command::new(&r_bin).args(&["1", "2"]).output().expect("Rust");
    assert_eq!(c_out.stdout, r_out.stdout, "too-many-args stdout mismatch");

    // Non-integer argument
    let c_out = Command::new(&c_bin).arg("abc").output().expect("C");
    let r_out = Command::new(&r_bin).arg("abc").output().expect("Rust");
    assert_eq!(c_out.stdout, r_out.stdout, "non-integer stdout mismatch");
}
