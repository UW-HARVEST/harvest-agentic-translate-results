use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

/// Paths to the shared libraries
fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libString_Slice.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libString_Slice.so")
}

type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);
    }
    f();
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush C stdout
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
    }
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let mut buf = String::new();
    reader.read_to_string(&mut buf).unwrap();
    buf
}

/// Call slice via a loaded library, capturing stdout and return code.
fn call_slice(
    lib: &Library,
    s: &str,
    start: Option<i32>,
    stop: Option<i32>,
) -> (i32, String) {
    let func: Symbol<SliceFn> = unsafe { lib.get(b"slice").unwrap() };
    let cstr = CString::new(s).unwrap();
    let mut start_val = start.unwrap_or(0);
    let mut stop_val = stop.unwrap_or(0);
    let start_ptr = if start.is_some() { &mut start_val as *mut c_int } else { std::ptr::null_mut() };
    let stop_ptr = if stop.is_some() { &mut stop_val as *mut c_int } else { std::ptr::null_mut() };

    let mut ret = 0i32;
    let output = capture_stdout(|| {
        ret = unsafe { func(cstr.as_ptr() as *mut c_char, start_ptr, stop_ptr) };
    });
    (ret, output)
}

macro_rules! compare {
    ($c_lib:expr, $r_lib:expr, $s:expr, $start:expr, $stop:expr) => {{
        let (c_ret, c_out) = call_slice($c_lib, $s, $start, $stop);
        let (r_ret, r_out) = call_slice($r_lib, $s, $start, $stop);
        assert_eq!(
            (c_ret, &c_out),
            (r_ret, &r_out),
            "Mismatch for s={:?} start={:?} stop={:?}",
            $s, $start, $stop
        );
    }};
}

#[test]
fn test_slice_basic() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Full string (both ptrs null)
    compare!(&c_lib, &r_lib, "hello", None::<i32>, None::<i32>);

    // With start only
    compare!(&c_lib, &r_lib, "hello", Some(2), None::<i32>);

    // With start and stop
    compare!(&c_lib, &r_lib, "hello", Some(1), Some(4));

    // Start = 0, stop = len
    compare!(&c_lib, &r_lib, "hello", Some(0), Some(5));

    // Single char slice
    compare!(&c_lib, &r_lib, "hello", Some(0), Some(1));

    // Empty string, no ptrs
    compare!(&c_lib, &r_lib, "", None::<i32>, None::<i32>);
}

#[test]
fn test_slice_errors() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Start past end
    compare!(&c_lib, &r_lib, "hi", Some(10), None::<i32>);

    // Stop past end
    compare!(&c_lib, &r_lib, "hi", Some(0), Some(10));

    // Stop <= start
    compare!(&c_lib, &r_lib, "hello", Some(3), Some(2));
    compare!(&c_lib, &r_lib, "hello", Some(3), Some(3));
}

#[test]
fn test_slice_edge_cases() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Start at 0 with stop
    compare!(&c_lib, &r_lib, "abcdef", Some(0), Some(3));

    // Start at last char
    compare!(&c_lib, &r_lib, "abcdef", Some(5), None::<i32>);

    // Stop at len (boundary)
    compare!(&c_lib, &r_lib, "abcdef", Some(0), Some(6));

    // Only stop, no start
    compare!(&c_lib, &r_lib, "abcdef", None::<i32>, Some(3));

    // Longer string
    compare!(&c_lib, &r_lib, "the quick brown fox", Some(4), Some(9));
}
