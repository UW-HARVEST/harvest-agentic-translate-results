use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn fflush(stream: *mut libc::c_void) -> c_int;
}

/// Capture stdout from a closure by dup'ing fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush before redirecting
    unsafe { fflush(std::ptr::null_mut()) }; // NULL flushes all streams

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };

    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1) };
    unsafe { libc::close(pipe_fds[1]) };

    f();

    // Flush both C and Rust stdout
    unsafe { fflush(std::ptr::null_mut()) };
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    let mut buf = String::new();
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdriver.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

#[test]
fn test_driver_output() {
    let lib = c_lib();
    let test_inputs: &[c_int] = &[0, 1, -1, 100, -100, i32::MAX / 2, i32::MIN / 2];

    for &x in test_inputs {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };

        let rust_out = capture_stdout(|| ::driver::driver(x));

        assert_eq!(
            c_out, rust_out,
            "Mismatch for x={x}: C={c_out:?}, Rust={rust_out:?}"
        );
    }
}
