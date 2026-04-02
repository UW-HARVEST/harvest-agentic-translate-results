use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout by redirecting fd 1 to a pipe.
/// Flushes both before and after the closure.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    // flush C stdio while fd 1 still points to pipe
    unsafe { libc::fflush(std::ptr::null_mut()); }
    std::io::stdout().flush().unwrap();

    unsafe {
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(p).unwrap() }
}

fn rust_lib() -> Library {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(p).unwrap() }
}

// Run all tests serially since we're redirecting fd 1
#[test]
fn test_all_functions() {
    // 1. printLine with a string
    {
        let c = c_lib();
        let r = rust_lib();
        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> = unsafe { c.get(b"printLine").unwrap() };
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> = unsafe { r.get(b"printLine").unwrap() };

        let msg = CString::new("hello world").unwrap();
        let c_out = capture_stdout(|| unsafe { c_fn(msg.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_fn(msg.as_ptr()) });
        assert_eq!(c_out, r_out, "printLine(\"hello world\") mismatch: C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }

    // 2. printLine with NULL
    {
        let c = c_lib();
        let r = rust_lib();
        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> = unsafe { c.get(b"printLine").unwrap() };
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> = unsafe { r.get(b"printLine").unwrap() };

        let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
        let r_out = capture_stdout(|| unsafe { r_fn(std::ptr::null()) });
        assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
    }

    // 3. good()
    {
        let c = c_lib();
        let r = rust_lib();
        let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"good").unwrap() };
        let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"good").unwrap() };

        let c_out = capture_stdout(|| unsafe { c_fn() });
        let r_out = capture_stdout(|| unsafe { r_fn() });
        assert_eq!(c_out, r_out, "good() mismatch: C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }

    // 4. driver(1) — the "good" path
    {
        let c = c_lib();
        let r = rust_lib();
        let c_fn: Symbol<unsafe extern "C" fn(c_int)> = unsafe { c.get(b"driver").unwrap() };
        let r_fn: Symbol<unsafe extern "C" fn(c_int)> = unsafe { r.get(b"driver").unwrap() };

        let c_out = capture_stdout(|| unsafe { c_fn(1) });
        let r_out = capture_stdout(|| unsafe { r_fn(1) });
        assert_eq!(c_out, r_out, "driver(1) mismatch: C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}
