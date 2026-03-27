use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush Rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    unsafe {
        let mut fds = [0 as libc_pipe_fd; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);

        let old_stdout = libc_dup(1);
        assert!(old_stdout >= 0);
        libc_dup2(fds[1], 1);
        libc_close(fds[1]);

        f();

        // Flush C stdout
        libc_fflush(std::ptr::null_mut());
        std::io::stdout().flush().unwrap();

        libc_dup2(old_stdout, 1);
        libc_close(old_stdout);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        // Set non-blocking read with a small buffer approach
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

type libc_pipe_fd = i32;

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    #[link_name = "dup"]
    fn libc_dup(fd: i32) -> i32;
    #[link_name = "dup2"]
    fn libc_dup2(oldfd: i32, newfd: i32) -> i32;
    #[link_name = "close"]
    fn libc_close(fd: i32) -> i32;
    #[link_name = "fflush"]
    fn libc_fflush(stream: *mut std::ffi::c_void) -> i32;
}

use std::os::unix::io::FromRawFd;

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_output_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver symbol") };

    let test_values: &[c_int] = &[0, 1, -1, 42, 0x7FFFFFFF, -0x7FFFFFFF, 0x12345678, 255];

    for &val in test_values {
        let c_output = capture_stdout(|| unsafe { c_driver(val) });
        let rust_output = capture_stdout(|| ::driver::driver(val));

        assert_eq!(
            c_output, rust_output,
            "Mismatch for input {val} (0x{val:08x}): C={c_output:?} Rust={rust_output:?}"
        );
    }
}
