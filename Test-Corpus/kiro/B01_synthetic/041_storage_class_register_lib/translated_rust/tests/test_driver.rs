use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by `f()` by dup'ing fd 1 into a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        // flush C and Rust stdout
        libc::fflush(std::ptr::null_mut());
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        // flush both layers again
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(pipe_fds[0]);
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src/build/libdriver.so"),
        )
        .expect("failed to load C .so")
    }
}

fn rust_lib() -> Library {
    // cargo puts cdylib in target/<profile>/deps or target/<profile>/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    let path = dir.join("libdriver.so");
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

#[test]
fn test_driver_matches() {
    let c = c_lib();
    let r = rust_lib();

    let test_values: &[c_int] = &[0, 1, -1, 100, -100, i32::MAX / 2, i32::MIN / 2, 42, 999];

    for &x in test_values {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        assert_eq!(
            c_out, r_out,
            "mismatch for x={x}: C={c_out:?} Rust={r_out:?}"
        );
    }
}
