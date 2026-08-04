use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::PathBuf;
use std::sync::Mutex;

type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

// Serialize stdout-capturing tests across threads, since stdout is a process-wide resource.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libString_Slice.so")
}

fn rust_lib_path() -> PathBuf {
    // Try debug first, then release.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug_path = manifest.join("target/debug/libString_Slice.so");
    let release_path = manifest.join("target/release/libString_Slice.so");
    if debug_path.exists() {
        debug_path
    } else {
        release_path
    }
}

/// Capture everything written to stdout by `f` (including from C printf via fd 1).
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure libc has flushed any buffered stdout content first.
    unsafe {
        // FILE* stdout flush
        extern "C" {
            fn fflush(stream: *mut std::ffi::c_void) -> c_int;
        }
        fflush(std::ptr::null_mut());
    }

    // Create a pipe.
    let mut fds: [c_int; 2] = [0; 2];
    unsafe {
        extern "C" {
            fn pipe(fds: *mut c_int) -> c_int;
            fn dup(fd: c_int) -> c_int;
            fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
            fn close(fd: c_int) -> c_int;
            fn fflush(stream: *mut std::ffi::c_void) -> c_int;
        }

        if pipe(fds.as_mut_ptr()) != 0 {
            panic!("pipe failed");
        }

        // Save old stdout.
        let saved = dup(1);
        if saved < 0 {
            panic!("dup failed");
        }

        // Redirect stdout to pipe write end.
        if dup2(fds[1], 1) < 0 {
            panic!("dup2 failed");
        }
        close(fds[1]);

        // Run the function.
        f();

        // Flush stdout so any buffered output makes it into the pipe.
        fflush(std::ptr::null_mut());

        // Restore stdout.
        if dup2(saved, 1) < 0 {
            panic!("dup2 restore failed");
        }
        close(saved);

        // Read all from pipe read end.
        let read_fd = fds[0];
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        // Read until EOF — pipe write end was closed by dup2 restore.
        // But we need to ensure write end is fully closed; since we only had two refs (fd 1 and fds[1]),
        // and dup2(saved,1) closed the old fd 1 (which was the pipe write end), it should be closed now.
        let _ = file.read_to_end(&mut buf);
        buf
    }
}

struct Libs {
    c_slice: SliceFn,
    rust_slice: SliceFn,
    _c_lib: Library,
    _rust_lib: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_lib_path()).expect("Failed to load C library");
            let rust_lib = Library::new(rust_lib_path()).expect("Failed to load Rust library");
            let c_slice: Symbol<SliceFn> = c_lib.get(b"slice").expect("slice in C lib");
            let rust_slice: Symbol<SliceFn> = rust_lib.get(b"slice").expect("slice in Rust lib");
            let c_slice = *c_slice;
            let rust_slice = *rust_slice;
            Libs {
                c_slice,
                rust_slice,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
            }
        }
    }
}

fn run_case(input: &str, start: Option<i32>, stop: Option<i32>) {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let libs = Libs::load();

    // Two separate buffers because each call mutates the input pointer
    // (well, slice doesn't mutate, but to be safe).
    let s_c = CString::new(input).unwrap();
    let s_r = CString::new(input).unwrap();

    let mut start_c = start.unwrap_or(0);
    let mut stop_c = stop.unwrap_or(0);
    let mut start_r = start.unwrap_or(0);
    let mut stop_r = stop.unwrap_or(0);

    let start_ptr_c = if start.is_some() { &mut start_c as *mut c_int } else { std::ptr::null_mut() };
    let stop_ptr_c = if stop.is_some() { &mut stop_c as *mut c_int } else { std::ptr::null_mut() };
    let start_ptr_r = if start.is_some() { &mut start_r as *mut c_int } else { std::ptr::null_mut() };
    let stop_ptr_r = if stop.is_some() { &mut stop_r as *mut c_int } else { std::ptr::null_mut() };

    let mut c_ret: c_int = 0;
    let mut r_ret: c_int = 0;

    let c_out = capture_stdout(|| unsafe {
        c_ret = (libs.c_slice)(s_c.as_ptr() as *mut c_char, start_ptr_c, stop_ptr_c);
    });

    let r_out = capture_stdout(|| unsafe {
        r_ret = (libs.rust_slice)(s_r.as_ptr() as *mut c_char, start_ptr_r, stop_ptr_r);
    });

    assert_eq!(
        c_ret, r_ret,
        "Return value mismatch for input={:?}, start={:?}, stop={:?}: C={}, Rust={}",
        input, start, stop, c_ret, r_ret
    );
    assert_eq!(
        c_out, r_out,
        "Stdout mismatch for input={:?}, start={:?}, stop={:?}\nC: {:?}\nRust: {:?}",
        input, start, stop,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

#[test]
fn test_full_string_no_args() {
    run_case("Hello, World!", None, None);
}

#[test]
fn test_with_start_only() {
    run_case("Hello, World!", Some(7), None);
}

#[test]
fn test_with_start_zero() {
    run_case("Hello, World!", Some(0), None);
}

#[test]
fn test_with_start_and_stop() {
    run_case("Hello, World!", Some(0), Some(5));
}

#[test]
fn test_with_stop_only() {
    run_case("Hello, World!", None, Some(5));
}

#[test]
fn test_start_at_end() {
    // start == len: in C, "if (start > len)" is false, so it proceeds.
    // stop is unset, so stop = len. printf prints stop-start = 0 chars.
    run_case("abc", Some(3), None);
}

#[test]
fn test_start_past_end() {
    run_case("abc", Some(4), None);
}

#[test]
fn test_stop_past_end() {
    run_case("abc", None, Some(4));
}

#[test]
fn test_stop_equal_start() {
    run_case("abcdef", Some(2), Some(2));
}

#[test]
fn test_stop_less_than_start() {
    run_case("abcdef", Some(4), Some(2));
}

#[test]
fn test_negative_start() {
    // Negative `start` cast to size_t becomes a very large unsigned value,
    // which exceeds len, hitting the error branch.
    run_case("abcdef", Some(-1), None);
}

#[test]
fn test_negative_stop() {
    run_case("abcdef", None, Some(-1));
}

#[test]
fn test_empty_string() {
    run_case("", None, None);
}

#[test]
fn test_empty_string_zero_zero() {
    // start = 0, stop = 0: stop <= start triggers error.
    run_case("", Some(0), Some(0));
}

#[test]
fn test_single_char_full() {
    run_case("X", None, None);
}

#[test]
fn test_substring_middle() {
    run_case("The quick brown fox", Some(4), Some(9));
}
