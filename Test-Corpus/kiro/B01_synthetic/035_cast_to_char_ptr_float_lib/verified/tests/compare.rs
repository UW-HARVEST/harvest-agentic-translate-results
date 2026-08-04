use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Call `driver(x)` inside the given library and capture its stdout output.
fn call_driver(lib: &Library, x: f32) -> String {
    unsafe {
        // Create a pipe to capture stdout
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);

        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        // Call the function
        let func: Symbol<unsafe extern "C" fn(f32)> = lib.get(b"driver").unwrap();
        func(x);

        // Flush C stdout
        libc::fflush(std::ptr::null_mut());

        // Restore stdout
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        // Read captured output
        let mut f = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn rust_so_path() -> std::path::PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::Path::new(&dir)
        .join("target/debug/libdriver.so")
}

fn c_so_path() -> std::path::PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::Path::new(&dir)
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_so_path()).unwrap() };
    let rust_lib = unsafe { Library::new(rust_so_path()).unwrap() };

    let test_values: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        3.14159265_f32,
        1e-38_f32,
        1e38_f32,
        42.0,
        0.1,
        std::f32::consts::E,
        std::f32::consts::PI,
    ];

    for &val in test_values {
        let c_out = call_driver(&c_lib, val);
        let rust_out = call_driver(&rust_lib, val);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for input {val}: C={c_out:?} Rust={rust_out:?}"
        );
    }
}
