use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug or target/release
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

/// Capture stdout+stderr from a closure by redirecting file descriptors.
/// Returns (return_value, stdout_bytes, stderr_bytes).
unsafe fn capture_output<F: FnOnce() -> T, T>(f: F) -> (T, Vec<u8>, Vec<u8>) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    // flush before redirecting
    libc::fflush(std::ptr::null_mut());

    // Create pipes for stdout and stderr
    let mut stdout_pipe = [0i32; 2];
    let mut stderr_pipe = [0i32; 2];
    libc::pipe(stdout_pipe.as_mut_ptr());
    libc::pipe(stderr_pipe.as_mut_ptr());

    let orig_stdout = libc::dup(1);
    let orig_stderr = libc::dup(2);

    libc::dup2(stdout_pipe[1], 1);
    libc::dup2(stderr_pipe[1], 2);
    libc::close(stdout_pipe[1]);
    libc::close(stderr_pipe[1]);

    let result = f();

    libc::fflush(std::ptr::null_mut());
    libc::dup2(orig_stdout, 1);
    libc::dup2(orig_stderr, 2);
    libc::close(orig_stdout);
    libc::close(orig_stderr);

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let mut stdout_file = std::fs::File::from_raw_fd(stdout_pipe[0]);
    let mut stderr_file = std::fs::File::from_raw_fd(stderr_pipe[0]);

    // Set non-blocking to avoid hanging if no data
    libc::fcntl(stdout_pipe[0], libc::F_SETFL, libc::O_NONBLOCK);
    libc::fcntl(stderr_pipe[0], libc::F_SETFL, libc::O_NONBLOCK);

    let _ = stdout_file.read_to_end(&mut stdout_buf);
    let _ = stderr_file.read_to_end(&mut stderr_buf);

    (result, stdout_buf, stderr_buf)
}

#[test]
fn test_forward_goto_example_positive() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(c_int) -> c_int;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"forward_goto_example").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"forward_goto_example").unwrap() };

    for x in [0, 1, 5, 42, 100, i32::MAX / 2] {
        let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(x)) };
        let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(x)) };
        assert_eq!(c_ret, r_ret, "return mismatch for x={x}");
        assert_eq!(c_out, r_out, "stdout mismatch for x={x}");
        assert_eq!(c_err, r_err, "stderr mismatch for x={x}");
    }
}

#[test]
fn test_forward_goto_example_negative() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(c_int) -> c_int;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"forward_goto_example").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"forward_goto_example").unwrap() };

    for x in [-1, -100, i32::MIN] {
        let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(x)) };
        let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(x)) };
        assert_eq!(c_ret, r_ret, "return mismatch for x={x}");
        assert_eq!(c_out, r_out, "stdout mismatch for x={x}");
        assert_eq!(c_err, r_err, "stderr mismatch for x={x}");
    }
}

#[test]
fn test_open_with_cleanup_nonexistent() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(*const c_char) -> *mut libc::FILE;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"open_with_cleanup").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"open_with_cleanup").unwrap() };

    let fname = CString::new("/tmp/__nonexistent_test_file_12345__").unwrap();

    let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(fname.as_ptr())) };
    let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(fname.as_ptr())) };

    assert!(c_ret.is_null(), "C should return NULL for nonexistent file");
    assert!(r_ret.is_null(), "Rust should return NULL for nonexistent file");
    assert_eq!(c_out, r_out, "stdout mismatch for nonexistent file");
    assert_eq!(c_err, r_err, "stderr mismatch for nonexistent file");
}

#[test]
fn test_open_with_cleanup_valid_file() {
    // Create a temp file with known content
    let tmp = "/tmp/__goto_test_valid_file__.txt";
    std::fs::write(tmp, "hello world\nline two\n").unwrap();

    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(*const c_char) -> *mut libc::FILE;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"open_with_cleanup").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"open_with_cleanup").unwrap() };

    let fname = CString::new(tmp).unwrap();

    let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(fname.as_ptr())) };
    // Close the returned FILE* to avoid leak
    if !c_ret.is_null() {
        unsafe { libc::fclose(c_ret); }
    }

    let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(fname.as_ptr())) };
    if !r_ret.is_null() {
        unsafe { libc::fclose(r_ret); }
    }

    assert_eq!(c_ret.is_null(), r_ret.is_null(), "null-ness mismatch");
    assert_eq!(c_out, r_out, "stdout mismatch for valid file");
    assert_eq!(c_err, r_err, "stderr mismatch for valid file");

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn test_driver_positive() {
    let tmp = "/tmp/__goto_test_driver_file__.txt";
    std::fs::write(tmp, "test content\n").unwrap();

    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"driver").unwrap() };

    let fname = CString::new(tmp).unwrap();

    let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(5, fname.as_ptr())) };
    let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(5, fname.as_ptr())) };

    assert_eq!(c_ret, r_ret, "return mismatch for driver(5, file)");
    assert_eq!(c_out, r_out, "stdout mismatch for driver(5, file)");
    assert_eq!(c_err, r_err, "stderr mismatch for driver(5, file)");

    let _ = std::fs::remove_file(tmp);
}

#[test]
fn test_driver_negative_num() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"driver").unwrap() };

    let fname = CString::new("/tmp/doesntmatter").unwrap();

    let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(-1, fname.as_ptr())) };
    let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(-1, fname.as_ptr())) };

    assert_eq!(c_ret, r_ret, "return mismatch for driver(-1, ...)");
    assert_eq!(c_out, r_out, "stdout mismatch for driver(-1, ...)");
    assert_eq!(c_err, r_err, "stderr mismatch for driver(-1, ...)");
}

#[test]
fn test_driver_bad_file() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FnType = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

    let c_fn: Symbol<FnType> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<FnType> = unsafe { rust_lib.get(b"driver").unwrap() };

    let fname = CString::new("/tmp/__nonexistent_driver_test__").unwrap();

    let (c_ret, c_out, c_err) = unsafe { capture_output(|| c_fn(5, fname.as_ptr())) };
    let (r_ret, r_out, r_err) = unsafe { capture_output(|| r_fn(5, fname.as_ptr())) };

    assert_eq!(c_ret, r_ret, "return mismatch for driver(5, bad_file)");
    assert_eq!(c_out, r_out, "stdout mismatch for driver(5, bad_file)");
    assert_eq!(c_err, r_err, "stderr mismatch for driver(5, bad_file)");
}
