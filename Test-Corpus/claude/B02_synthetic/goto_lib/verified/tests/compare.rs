use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize tests because we manipulate process-global stdout/stderr fds.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Use the release target for testing the actual exported .so
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
}

/// Capture stdout and stderr produced by `f`.
/// We flush, dup the existing fds, replace 1 and 2 with pipes -> tmp files,
/// run, flush again, then restore.
fn capture_output<F: FnOnce()>(f: F) -> (Vec<u8>, Vec<u8>) {
    unsafe {
        // Flush libc-level FILE buffers before swapping fds.
        libc::fflush(std::ptr::null_mut());

        let tmp_dir = std::env::temp_dir();
        let stdout_path = tmp_dir.join(format!(
            "harvest_stdout_{}_{:p}.txt",
            std::process::id(),
            &f as *const _
        ));
        let stderr_path = tmp_dir.join(format!(
            "harvest_stderr_{}_{:p}.txt",
            std::process::id(),
            &f as *const _
        ));

        let stdout_file = std::fs::File::create(&stdout_path).unwrap();
        let stderr_file = std::fs::File::create(&stderr_path).unwrap();

        let saved_stdout = libc::dup(1);
        let saved_stderr = libc::dup(2);
        assert!(saved_stdout >= 0);
        assert!(saved_stderr >= 0);

        assert!(libc::dup2(stdout_file.as_raw_fd(), 1) >= 0);
        assert!(libc::dup2(stderr_file.as_raw_fd(), 2) >= 0);

        f();

        // Flush all libc FILE streams so output reaches our redirected fds.
        libc::fflush(std::ptr::null_mut());

        // Restore.
        assert!(libc::dup2(saved_stdout, 1) >= 0);
        assert!(libc::dup2(saved_stderr, 2) >= 0);
        libc::close(saved_stdout);
        libc::close(saved_stderr);

        drop(stdout_file);
        drop(stderr_file);

        let stdout_bytes = fs::read(&stdout_path).unwrap_or_default();
        let stderr_bytes = fs::read(&stderr_path).unwrap_or_default();
        let _ = fs::remove_file(&stdout_path);
        let _ = fs::remove_file(&stderr_path);

        (stdout_bytes, stderr_bytes)
    }
}

type ForwardGotoFn = unsafe extern "C" fn(c_int) -> c_int;
type OpenWithCleanupFn = unsafe extern "C" fn(*const c_char) -> *mut libc::FILE;
type DriverFn = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

#[test]
fn test_forward_goto_example_positive() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<ForwardGotoFn> = c_lib.get(b"forward_goto_example").unwrap();
        let r_fn: Symbol<ForwardGotoFn> = r_lib.get(b"forward_goto_example").unwrap();

        for x in [0, 1, 2, 7, 100, 12345, i32::MAX / 4] {
            let mut c_ret = 0;
            let mut r_ret = 0;
            let (c_out, c_err) = capture_output(|| c_ret = c_fn(x));
            let (r_out, r_err) = capture_output(|| r_ret = r_fn(x));

            assert_eq!(c_ret, r_ret, "ret mismatch for x={}", x);
            assert_eq!(c_out, r_out, "stdout mismatch for x={}", x);
            assert_eq!(c_err, r_err, "stderr mismatch for x={}", x);
        }
    }
}

#[test]
fn test_forward_goto_example_negative() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<ForwardGotoFn> = c_lib.get(b"forward_goto_example").unwrap();
        let r_fn: Symbol<ForwardGotoFn> = r_lib.get(b"forward_goto_example").unwrap();

        for x in [-1, -2, -100, i32::MIN] {
            let mut c_ret = 0;
            let mut r_ret = 0;
            let (c_out, c_err) = capture_output(|| c_ret = c_fn(x));
            let (r_out, r_err) = capture_output(|| r_ret = r_fn(x));

            assert_eq!(c_ret, r_ret, "ret mismatch for x={}", x);
            assert_eq!(c_out, r_out, "stdout mismatch for x={}", x);
            assert_eq!(c_err, r_err, "stderr mismatch for x={}", x);
        }
    }
}

#[test]
fn test_open_with_cleanup_existing_file() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<OpenWithCleanupFn> = c_lib.get(b"open_with_cleanup").unwrap();
        let r_fn: Symbol<OpenWithCleanupFn> = r_lib.get(b"open_with_cleanup").unwrap();

        let path = std::env::temp_dir().join("harvest_owc_existing.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "line one").unwrap();
            writeln!(f, "line two").unwrap();
            writeln!(f, "third line is here").unwrap();
        }
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        let mut c_ret: *mut libc::FILE = std::ptr::null_mut();
        let mut r_ret: *mut libc::FILE = std::ptr::null_mut();
        let (c_out, c_err) = capture_output(|| c_ret = c_fn(cstr.as_ptr()));
        let (r_out, r_err) = capture_output(|| r_ret = r_fn(cstr.as_ptr()));

        // Both should be non-null (or both null). Close ours.
        assert_eq!(c_ret.is_null(), r_ret.is_null(), "null-ness mismatch");
        if !c_ret.is_null() {
            libc::fclose(c_ret);
        }
        if !r_ret.is_null() {
            libc::fclose(r_ret);
        }

        assert_eq!(c_out, r_out, "stdout mismatch");
        assert_eq!(c_err, r_err, "stderr mismatch");

        let _ = fs::remove_file(&path);
    }
}

#[test]
fn test_open_with_cleanup_missing_file() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<OpenWithCleanupFn> = c_lib.get(b"open_with_cleanup").unwrap();
        let r_fn: Symbol<OpenWithCleanupFn> = r_lib.get(b"open_with_cleanup").unwrap();

        let path = std::env::temp_dir().join("harvest_owc_missing_xyz123.txt");
        let _ = fs::remove_file(&path);
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        let mut c_ret: *mut libc::FILE = std::ptr::null_mut();
        let mut r_ret: *mut libc::FILE = std::ptr::null_mut();
        let (c_out, c_err) = capture_output(|| c_ret = c_fn(cstr.as_ptr()));
        let (r_out, r_err) = capture_output(|| r_ret = r_fn(cstr.as_ptr()));

        assert!(c_ret.is_null());
        assert!(r_ret.is_null());

        assert_eq!(c_out, r_out, "stdout mismatch");
        assert_eq!(c_err, r_err, "stderr mismatch");
    }
}

#[test]
fn test_open_with_cleanup_empty_file() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<OpenWithCleanupFn> = c_lib.get(b"open_with_cleanup").unwrap();
        let r_fn: Symbol<OpenWithCleanupFn> = r_lib.get(b"open_with_cleanup").unwrap();

        let path = std::env::temp_dir().join("harvest_owc_empty.txt");
        std::fs::File::create(&path).unwrap();
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        let mut c_ret: *mut libc::FILE = std::ptr::null_mut();
        let mut r_ret: *mut libc::FILE = std::ptr::null_mut();
        let (c_out, c_err) = capture_output(|| c_ret = c_fn(cstr.as_ptr()));
        let (r_out, r_err) = capture_output(|| r_ret = r_fn(cstr.as_ptr()));

        assert_eq!(c_ret.is_null(), r_ret.is_null());
        if !c_ret.is_null() {
            libc::fclose(c_ret);
        }
        if !r_ret.is_null() {
            libc::fclose(r_ret);
        }

        assert_eq!(c_out, r_out, "stdout mismatch");
        assert_eq!(c_err, r_err, "stderr mismatch");

        let _ = fs::remove_file(&path);
    }
}

#[test]
fn test_driver_positive_existing() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let path = std::env::temp_dir().join("harvest_driver_ok.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "hello world").unwrap();
            writeln!(f, "second line").unwrap();
        }
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        for num in [0, 1, 5, 42] {
            let mut c_ret = 0;
            let mut r_ret = 0;
            let (c_out, c_err) = capture_output(|| c_ret = c_fn(num, cstr.as_ptr()));
            let (r_out, r_err) = capture_output(|| r_ret = r_fn(num, cstr.as_ptr()));

            assert_eq!(c_ret, r_ret, "ret mismatch for num={}", num);
            assert_eq!(c_out, r_out, "stdout mismatch for num={}", num);
            assert_eq!(c_err, r_err, "stderr mismatch for num={}", num);
        }

        let _ = fs::remove_file(&path);
    }
}

#[test]
fn test_driver_negative() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let path = std::env::temp_dir().join("harvest_driver_neg.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "ignored content").unwrap();
        }
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        for num in [-1, -7, i32::MIN] {
            let mut c_ret = 0;
            let mut r_ret = 0;
            let (c_out, c_err) = capture_output(|| c_ret = c_fn(num, cstr.as_ptr()));
            let (r_out, r_err) = capture_output(|| r_ret = r_fn(num, cstr.as_ptr()));

            assert_eq!(c_ret, r_ret, "ret mismatch for num={}", num);
            assert_eq!(c_out, r_out, "stdout mismatch for num={}", num);
            assert_eq!(c_err, r_err, "stderr mismatch for num={}", num);
        }

        let _ = fs::remove_file(&path);
    }
}

#[test]
fn test_driver_positive_missing_file() {
    let _g = TEST_LOCK.lock().unwrap();
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let path = std::env::temp_dir().join("harvest_driver_no_such_file_zzz.txt");
        let _ = fs::remove_file(&path);
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        for num in [0, 3] {
            let mut c_ret = 0;
            let mut r_ret = 0;
            let (c_out, c_err) = capture_output(|| c_ret = c_fn(num, cstr.as_ptr()));
            let (r_out, r_err) = capture_output(|| r_ret = r_fn(num, cstr.as_ptr()));

            assert_eq!(c_ret, r_ret, "ret mismatch for num={}", num);
            assert_eq!(c_out, r_out, "stdout mismatch for num={}", num);
            assert_eq!(c_err, r_err, "stderr mismatch for num={}", num);
        }
    }
}
