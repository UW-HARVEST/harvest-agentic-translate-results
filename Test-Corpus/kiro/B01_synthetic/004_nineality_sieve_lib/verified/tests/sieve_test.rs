use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `sieve(val)` from the given .so library.
fn capture_sieve(lib_path: &str, val: i32) -> String {
    // Create a pipe to capture stdout
    let mut fds = [0i32; 2];
    unsafe { assert_eq!(libc_pipe(fds.as_mut_ptr()), 0) };
    let (read_fd, write_fd) = (fds[0], fds[1]);

    // Save original stdout, redirect stdout to pipe write end
    let orig_stdout = unsafe { libc_dup(1) };
    unsafe { libc_dup2(write_fd, 1) };

    // Load library and call sieve
    unsafe {
        let lib = Library::new(lib_path).expect("failed to load library");
        let func: Symbol<unsafe extern "C" fn(i32)> =
            lib.get(b"sieve").expect("failed to find sieve symbol");
        func(val);
        // Flush C stdout in case the C .so uses printf with buffering
        libc_fflush(std::ptr::null_mut());
    }

    // Restore original stdout, close write end
    unsafe {
        libc_dup2(orig_stdout, 1);
        libc_close(orig_stdout);
        libc_close(write_fd);
    }

    // Read captured output
    let mut output = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    reader.read_to_string(&mut output).unwrap();
    output
}

// Minimal libc FFI for pipe/dup/dup2/close/fflush
extern "C" {
    #[link_name = "pipe"]
    fn libc_pipe(fds: *mut i32) -> i32;
    #[link_name = "dup"]
    fn libc_dup(fd: i32) -> i32;
    #[link_name = "dup2"]
    fn libc_dup2(old: i32, new: i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
    #[link_name = "fflush"]
    fn libc_fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn c_lib_path() -> String {
    std::env::var("C_LIB_PATH").unwrap_or_else(|_| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{}/c_src/build/libSieve.so", manifest)
    })
}

fn rust_lib_path() -> String {
    std::env::var("RUST_LIB_PATH").unwrap_or_else(|_| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        // Find the .so in target/debug
        let p = format!("{}/target/debug/libSieve.so", manifest);
        if std::path::Path::new(&p).exists() {
            return p;
        }
        format!("{}/target/release/libSieve.so", manifest)
    })
}

#[test]
fn test_sieve_outputs_match() {
    let c_lib = c_lib_path();
    let r_lib = rust_lib_path();

    // Test a variety of starting values
    let test_values = [0, 1, 5, 9, 10, 15, 19, 99, 100, -1, -10, -11];

    for &val in &test_values {
        let c_out = capture_sieve(&c_lib, val);
        let r_out = capture_sieve(&r_lib, val);
        assert_eq!(
            c_out.as_bytes(),
            r_out.as_bytes(),
            "Mismatch for sieve({})\nC output:    {:?}\nRust output: {:?}",
            val,
            c_out,
            r_out
        );
    }
}
