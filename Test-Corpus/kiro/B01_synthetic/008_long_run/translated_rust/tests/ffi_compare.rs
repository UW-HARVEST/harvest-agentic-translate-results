use libloading::{Library, Symbol};
use std::path::PathBuf;

const ARRAY_SIZE: usize = 256 * 1024;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

/// Test perform_expensive_operations: set identical array contents, call both, compare.
#[test]
fn test_perform_expensive_operations() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_array: Symbol<*mut [i32; ARRAY_SIZE]> = c_lib.get(b"array").unwrap();
        let r_array: Symbol<*mut [i32; ARRAY_SIZE]> = rust_lib.get(b"array").unwrap();

        let c_perform: Symbol<unsafe extern "C" fn()> =
            c_lib.get(b"perform_expensive_operations").unwrap();
        let r_perform: Symbol<unsafe extern "C" fn()> =
            rust_lib.get(b"perform_expensive_operations").unwrap();

        // Fill both arrays with the same deterministic data
        for i in 0..ARRAY_SIZE {
            (**c_array)[i] = (i as i32).wrapping_mul(17).wrapping_add(3);
            (**r_array)[i] = (i as i32).wrapping_mul(17).wrapping_add(3);
        }

        c_perform();
        r_perform();

        let c_slice: &[i32; ARRAY_SIZE] = &**c_array;
        let r_slice: &[i32; ARRAY_SIZE] = &**r_array;
        assert_eq!(c_slice, r_slice, "perform_expensive_operations mismatch");
    }
}

/// Test full pipeline by running both executables and comparing stdout.
/// Uses release build for speed since 2000 iterations * 256K is heavy.
#[test]
fn test_main_output_matches() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_exe = manifest.join("c_src/build/driver");

    // Build Rust in release mode for this test
    let status = std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&manifest)
        .status()
        .expect("cargo build --release");
    assert!(status.success(), "Rust release build failed");
    let rust_exe = manifest.join("target/release/driver");

    for seed in [0u32, 1, 42] {
        let c_out = std::process::Command::new(&c_exe)
            .arg(seed.to_string())
            .output()
            .expect("run C exe");
        let r_out = std::process::Command::new(&rust_exe)
            .arg(seed.to_string())
            .output()
            .expect("run Rust exe");

        assert_eq!(
            String::from_utf8_lossy(&c_out.stdout),
            String::from_utf8_lossy(&r_out.stdout),
            "Output mismatch for seed {seed}"
        );
    }
}
