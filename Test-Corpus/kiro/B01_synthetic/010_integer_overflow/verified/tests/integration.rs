use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", dir)
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    // flush after call
    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_print_hex_char_line() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let test_values: Vec<c_char> = vec![0, 1, 32, 65, 127, -1, -128, -2, 100];

    for &val in &test_values {
        let c_out = {
            let func: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { c_lib.get(b"printHexCharLine").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };
        let rust_out = {
            let func: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { rust_lib.get(b"printHexCharLine").unwrap() };
            capture_stdout(|| unsafe { func(val) })
        };
        assert_eq!(
            c_out, rust_out,
            "printHexCharLine({}) mismatch: C={:?} Rust={:?}",
            val, c_out, rust_out
        );
    }
}

#[test]
fn test_exports_match() {
    // Verify both libraries export the same user symbols
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    for sym in &[b"printHexCharLine" as &[u8], b"main"] {
        unsafe {
            let _c: Symbol<unsafe extern "C" fn()> =
                c_lib.get(sym).unwrap_or_else(|_| panic!("C missing {:?}", std::str::from_utf8(sym)));
            let _r: Symbol<unsafe extern "C" fn()> =
                rust_lib.get(sym).unwrap_or_else(|_| panic!("Rust missing {:?}", std::str::from_utf8(sym)));
        }
    }
}
