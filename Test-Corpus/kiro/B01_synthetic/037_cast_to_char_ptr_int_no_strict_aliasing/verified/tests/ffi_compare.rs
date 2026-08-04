use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::Read;

extern "C" {
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe { fflush(std::ptr::null_mut()); }

    let mut pipes = [0i32; 2];
    unsafe { pipe(pipes.as_mut_ptr()); }
    let (read_fd, write_fd) = (pipes[0], pipes[1]);

    let saved = unsafe { dup(1) };
    unsafe { dup2(write_fd, 1); }

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        close(write_fd);
    }

    let mut result = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    reader.read_to_string(&mut result).unwrap();
    result
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver_c.so");
    unsafe { Library::new(&path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

#[test]
fn test_driver_matches() {
    let c = c_lib();
    let r = rust_lib();

    let test_values: &[i32] = &[
        0, 1, -1, 42, 255, 256, 65535, -2147483648, 2147483647,
        0x01020304, -42, 0x7f, 0x80, 0xff,
    ];

    for &x in test_values {
        let c_output = {
            let f: Symbol<unsafe extern "C" fn(i32)> = unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        let r_output = {
            let f: Symbol<unsafe extern "C" fn(i32)> = unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        assert_eq!(
            c_output, r_output,
            "Mismatch for driver({}): C={:?} Rust={:?}", x, c_output, r_output
        );
    }
}
