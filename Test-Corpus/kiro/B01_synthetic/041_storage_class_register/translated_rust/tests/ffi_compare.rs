use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `f()` by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // flush Rust stdout first
    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut pipe_fds = [0 as c_int; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    let (pipe_r, pipe_w) = (pipe_fds[0], pipe_fds[1]);

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_w, 1) };
    unsafe { libc::close(pipe_w) };

    f();

    // flush libc stdout so printf output lands in the pipe
    unsafe { libc::fflush(std::ptr::null_mut()) };
    // restore original stdout
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_r) };
    reader.read_to_end(&mut buf).ok();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");

    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver") }.expect("C driver symbol");
    let r_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { r_lib.get(b"driver") }.expect("Rust driver symbol");

    let test_values: &[i32] = &[
        0, 1, -1, 100, -100, i32::MAX, i32::MIN, 42, 999999, -999999,
    ];

    for &x in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(x) });
        let r_out = capture_stdout(|| unsafe { r_driver(x) });
        assert_eq!(
            c_out, r_out,
            "Mismatch for x={}: C={:?} Rust={:?}",
            x,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
