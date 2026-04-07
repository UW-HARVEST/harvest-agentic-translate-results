use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    static stdout: *mut libc::FILE;
}

/// Capture stdout produced by `f()` by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // Flush before redirecting
    unsafe { libc::fflush(stdout) };

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let (read_fd, write_fd) = (fds[0], fds[1]);

    let saved = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1) };

    f();

    // Flush after call
    unsafe { libc::fflush(stdout) };
    unsafe { libc::dup2(saved, 1) };
    unsafe { libc::close(saved) };
    unsafe { libc::close(write_fd) };

    let mut buf = String::new();
    unsafe { std::fs::File::from_raw_fd(read_fd) }
        .read_to_string(&mut buf)
        .unwrap();
    buf
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");
    unsafe { Library::new(path).expect("failed to load Rust .so") }
}

fn call_driver(lib: &Library, x: c_int, y: c_int) -> String {
    capture_stdout(|| unsafe {
        let func: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            lib.get(b"driver").expect("symbol not found");
        func(x, y);
    })
}

#[test]
fn test_driver_outputs_match() {
    let c = c_lib();
    let r = rust_lib();

    let cases: &[(c_int, c_int)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (1, 4),
        (2, 1),
        (2, 2),
        (3, 1),
        (3, 3),
        (4, 2),
        (2, 4),
        (3, 4),
        (5, 5),
        (0, 3),
        (1, 2),
        (2, 3),
        (4, 4),
    ];

    for &(x, y) in cases {
        let c_out = call_driver(&c, x, y);
        let r_out = call_driver(&r, x, y);
        assert_eq!(
            c_out, r_out,
            "MISMATCH for driver({}, {})\n--- C output ---\n{}\n--- Rust output ---\n{}",
            x, y, c_out, r_out
        );
    }
}
