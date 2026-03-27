use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libStaticAlias.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    // Try debug first
    let debug = path.join("debug").join("libStaticAlias.so");
    if debug.exists() {
        return debug;
    }
    path.join("release").join("libStaticAlias.so")
}

/// Test static_alias: call with a sequence of inputs and compare C vs Rust results.
/// Both libraries are loaded fresh so their static state starts at inner=1.
#[test]
fn test_static_alias_sequence() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C lib");
    let rust_lib = unsafe { Library::new(rust_lib_path()) }.expect("Failed to load Rust lib");

    type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;

    let c_fn: Symbol<StaticAliasFn> =
        unsafe { c_lib.get(b"static_alias") }.expect("C static_alias not found");
    let rust_fn: Symbol<StaticAliasFn> =
        unsafe { rust_lib.get(b"static_alias") }.expect("Rust static_alias not found");

    // Test with several sequences of outer values
    let test_values: Vec<c_int> = vec![5, 3, 10, 2, 20, 1, 0, 100];

    let mut c_outer_vals = test_values.clone();
    let mut rust_outer_vals = test_values.clone();

    for i in 0..test_values.len() {
        let c_result = unsafe { *c_fn(&mut c_outer_vals[i]) };
        let rust_result = unsafe { *rust_fn(&mut rust_outer_vals[i]) };

        assert_eq!(
            c_result, rust_result,
            "static_alias mismatch at call {}: C returned {}, Rust returned {} (input was {})",
            i, c_result, rust_result, test_values[i]
        );
        // Also compare the outer value (it may have been modified)
        assert_eq!(
            c_outer_vals[i], rust_outer_vals[i],
            "static_alias outer mismatch at call {}: C outer={}, Rust outer={}",
            i, c_outer_vals[i], rust_outer_vals[i]
        );
    }
}

/// Test driver by capturing stdout from both C and Rust versions via a child process approach.
/// We use pipe/dup2 to capture printf output.
#[test]
fn test_driver_output() {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    // Helper: call driver in a lib, capturing stdout
    fn capture_driver(lib_path: PathBuf, initial_value: c_int, iterations: c_int) -> String {
        let lib = unsafe { Library::new(&lib_path) }.expect("Failed to load lib");
        type DriverFn = unsafe extern "C" fn(c_int, c_int);
        let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver") }.expect("driver not found");

        // Create a pipe
        let mut fds = [0i32; 2];
        unsafe { libc::pipe(fds.as_mut_ptr()) };
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Save original stdout
        let saved_stdout = unsafe { libc::dup(1) };

        // Redirect stdout to pipe write end
        unsafe { libc::dup2(write_fd, 1) };
        unsafe { libc::close(write_fd) };

        // Flush before calling
        unsafe { libc::fflush(std::ptr::null_mut()) };

        // Call driver
        unsafe { driver(initial_value, iterations) };

        // Flush and restore stdout
        unsafe { libc::fflush(std::ptr::null_mut()) };
        unsafe { libc::dup2(saved_stdout, 1) };
        unsafe { libc::close(saved_stdout) };

        // Read captured output
        let mut output = String::new();
        let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        file.read_to_string(&mut output).unwrap();
        output
    }

    let c_output = capture_driver(c_lib_path(), 5, 6);
    let rust_output = capture_driver(rust_lib_path(), 5, 6);

    assert_eq!(
        c_output, rust_output,
        "driver output mismatch:\nC output:\n{}\nRust output:\n{}",
        c_output, rust_output
    );
}
