use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout from a closure by dup2-ing fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush Rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut pipe_fds = [0 as c_int; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); }

    // also flush C stdout
    unsafe { libc::fflush(std::ptr::null_mut()); }

    f();

    // flush both
    unsafe { libc::fflush(std::ptr::null_mut()); }
    std::io::stdout().flush().unwrap();

    unsafe { libc::dup2(saved_stdout, 1); }
    unsafe { libc::close(write_fd); }
    unsafe { libc::close(saved_stdout); }

    let mut result = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    reader.read_to_string(&mut result).unwrap();
    result
}

#[repr(C)]
struct FooT {
    _bitfield: u32,
    z: i32,
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // cdylib output
    format!("{}/target/debug/libdriver.so", manifest)
}

#[test]
fn test_driver_basic() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let test_cases: Vec<(u32, u32, bool, i32)> = vec![
        (0, 0, false, 0),
        (1, 2, true, 42),
        (3, 7, true, -1),
        (3, 7, false, 100),
        (5, 10, true, 999),  // values exceeding bitfield widths
        (0, 0, true, i32::MIN),
        (3, 7, false, i32::MAX),
    ];

    for (x, y, b, z) in &test_cases {
        let c_output = capture_stdout(|| unsafe {
            let func: Symbol<unsafe extern "C" fn(u32, u32, bool, i32)> =
                c_lib.get(b"driver").unwrap();
            func(*x, *y, *b, *z);
        });

        let rust_output = capture_stdout(|| unsafe {
            let func: Symbol<unsafe extern "C" fn(u32, u32, bool, i32)> =
                rust_lib.get(b"driver").unwrap();
            func(*x, *y, *b, *z);
        });

        assert_eq!(
            c_output, rust_output,
            "driver({}, {}, {}, {}): C='{}' Rust='{}'",
            x, y, b, z, c_output.trim(), rust_output.trim()
        );
    }
}

#[test]
fn test_print_foo() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let test_cases: Vec<(u32, u32, bool, i32)> = vec![
        (0, 0, false, 0),
        (1, 2, true, 42),
        (3, 7, true, -1),
        (2, 5, false, 100),
    ];

    for (x, y, b, z) in &test_cases {
        // Build the struct the same way C does
        let mut bf: u32 = 0;
        bf |= x & 0x3;
        bf |= (y & 0x7) << 2;
        bf |= (*b as u32) << 5;
        let foo = FooT { _bitfield: bf, z: *z };

        let c_output = capture_stdout(|| unsafe {
            let func: Symbol<unsafe extern "C" fn(*const FooT)> =
                c_lib.get(b"print_foo").unwrap();
            func(&foo as *const FooT);
        });

        let rust_output = capture_stdout(|| unsafe {
            let func: Symbol<unsafe extern "C" fn(*const FooT)> =
                rust_lib.get(b"print_foo").unwrap();
            func(&foo as *const FooT);
        });

        assert_eq!(
            c_output, rust_output,
            "print_foo({}, {}, {}, {}): C='{}' Rust='{}'",
            x, y, b, z, c_output.trim(), rust_output.trim()
        );
    }
}
