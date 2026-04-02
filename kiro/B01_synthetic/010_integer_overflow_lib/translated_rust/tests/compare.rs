use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

/// Capture output written to fd 1 (works for both C printf and Rust println!
/// when run with --nocapture so Rust's println! goes to real fd 1).
fn capture_fd1<F: FnOnce()>(f: F) -> String {
    unsafe { libc::fflush(std::ptr::null_mut()); }
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let saved = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);
    }

    f();

    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe {
        libc::dup2(saved, 1);
        libc::close(saved);
    }

    let mut buf = String::new();
    unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) }
        .read_to_string(&mut buf)
        .unwrap();
    buf
}

const TEST_CHARS: &[i8] = &[0, 1, 42, 0x7f, -128, -1, -2, 100, -100];

#[test]
#[allow(non_snake_case)]
fn test_printHexCharLine() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { c_lib.get(b"printHexCharLine").unwrap() };

    for &val in TEST_CHARS {
        let c_out = capture_fd1(|| unsafe { c_fn(val as c_char) });
        let rust_out = capture_fd1(|| driver::printHexCharLine(val as c_char));
        assert_eq!(
            c_out, rust_out,
            "printHexCharLine mismatch for input {} (0x{:02x}): C={:?} Rust={:?}",
            val, val as u8, c_out, rust_out
        );
    }
}

#[test]
fn test_driver() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(c_char)> =
        unsafe { c_lib.get(b"driver").unwrap() };

    for &val in TEST_CHARS {
        let c_out = capture_fd1(|| unsafe { c_fn(val as c_char) });
        let rust_out = capture_fd1(|| driver::driver(val as c_char));
        assert_eq!(
            c_out, rust_out,
            "driver mismatch for input {} (0x{:02x}): C={:?} Rust={:?}",
            val, val as u8, c_out, rust_out
        );
    }
}
