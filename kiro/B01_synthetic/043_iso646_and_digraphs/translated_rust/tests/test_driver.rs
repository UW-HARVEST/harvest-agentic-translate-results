use libloading::{Library, Symbol};
use std::os::unix::io::FromRawFd;
use std::io::Read;

/// Capture stdout output from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(pipefd[1], 1);
        f();
        libc::fflush(std::ptr::null_mut()); // flush C stdout
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipefd[1]);
        let mut reader = std::fs::File::from_raw_fd(pipefd[0]);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target/debug/libdriver.so");
    path
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust lib") };

    let test_cases: &[(i32, i32)] = &[
        (0, 0),
        (1, 1),
        (-1, 0),
        (0, -1),
        (255, 128),
        (i32::MAX, i32::MIN),
        (12345, 67890),
        (-42, 42),
    ];

    for &(x, y) in test_cases {
        let c_output = {
            let func: Symbol<unsafe extern "C" fn(i32, i32)> =
                unsafe { c_lib.get(b"driver").expect("C driver not found") };
            capture_stdout(|| unsafe { func(x, y) })
        };

        let rust_output = {
            let func: Symbol<unsafe extern "C" fn(i32, i32)> =
                unsafe { rust_lib.get(b"driver").expect("Rust driver not found") };
            capture_stdout(|| unsafe { func(x, y) })
        };

        assert_eq!(
            c_output, rust_output,
            "Mismatch for driver({}, {}): C='{}' Rust='{}'",
            x, y, c_output, rust_output
        );
    }
}
