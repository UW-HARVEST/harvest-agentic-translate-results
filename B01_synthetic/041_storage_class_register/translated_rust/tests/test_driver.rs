use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::Read;

/// Capture stdout from a function call by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1); }
    f();
    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(old_stdout, 1); }
    unsafe { libc::close(old_stdout); }
    unsafe { libc::close(pipes[1]); }
    let mut buf = String::new();
    let mut read_end = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    read_end.read_to_string(&mut buf).unwrap();
    buf
}

fn rust_so_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try debug first (test profile), then release
    let debug = manifest.join("target/debug/libstorage_class_register.so");
    if debug.exists() { return debug; }
    manifest.join("target/release/libstorage_class_register.so")
}

#[test]
fn test_driver_matches() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build_shared/libdriver.so");
    let c_lib = unsafe { Library::new(&c_lib_path).expect("Failed to load C .so") };
    let c_driver: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver") };

    let rust_lib = unsafe { Library::new(rust_so_path()).expect("Failed to load Rust .so") };
    let rust_driver: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { rust_lib.get(b"driver").expect("Failed to find Rust driver") };

    let test_inputs: &[i32] = &[0, 1, -1, 100, -100, i32::MAX, i32::MIN, 42, 999999];

    for &x in test_inputs {
        let c_output = capture_stdout(|| unsafe { c_driver(x) });
        let rust_output = capture_stdout(|| unsafe { rust_driver(x) });
        assert_eq!(
            c_output.as_bytes(),
            rust_output.as_bytes(),
            "Mismatch for driver({}): C={:?} Rust={:?}",
            x, c_output, rust_output
        );
    }
}
