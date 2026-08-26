use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int};

type MainFn = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver_c.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

/// Call a `main(argc, argv)` function from a shared library, capturing stdout.
/// Returns (exit_code, captured_stdout).
fn call_main(lib: &Library, args: &[&str]) -> (i32, String) {
    let func: Symbol<MainFn> = unsafe { lib.get(b"main").unwrap() };

    let c_args: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
    let c_ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();

    // Create a pipe to capture stdout
    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    // Save original stdout and redirect
    let orig_stdout = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(write_fd, 1);
        libc::close(write_fd);
    }

    let ret = unsafe { func(c_ptrs.len() as c_int, c_ptrs.as_ptr()) };

    // Flush C stdout and Rust stdout
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();

    // Restore stdout
    unsafe {
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);
    }

    // Read captured output
    let mut output = String::new();
    let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    // Set non-blocking to avoid hanging
    unsafe {
        let flags = libc::fcntl(read_fd, libc::F_GETFL);
        libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let _ = read_file.read_to_string(&mut output);

    (ret, output)
}

use std::os::unix::io::FromRawFd;

fn compare(args: &[&str]) {
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("Failed to load Rust lib") };

    let (c_ret, c_out) = call_main(&c_lib, args);
    let (r_ret, r_out) = call_main(&rust_lib, args);

    assert_eq!(c_ret, r_ret, "Exit code mismatch for args {:?}: C={}, Rust={}", args, c_ret, r_ret);
    assert_eq!(c_out, r_out, "Output mismatch for args {:?}:\nC:    {:?}\nRust: {:?}", args, c_out, r_out);
}

// --- Test cases ---

#[test]
fn test_no_args() {
    compare(&["driver"]);
}

#[test]
fn test_too_many_args() {
    compare(&["driver", "hello", "1", "3", "extra"]);
}

#[test]
fn test_string_only() {
    compare(&["driver", "hello"]);
}

#[test]
fn test_string_with_start() {
    compare(&["driver", "hello", "2"]);
}

#[test]
fn test_string_with_start_and_stop() {
    compare(&["driver", "hello", "1", "4"]);
}

#[test]
fn test_start_zero() {
    compare(&["driver", "hello", "0"]);
}

#[test]
fn test_start_zero_stop_full() {
    compare(&["driver", "hello", "0", "5"]);
}

#[test]
fn test_start_equals_stop() {
    compare(&["driver", "hello", "2", "2"]);
}

#[test]
fn test_stop_before_start() {
    compare(&["driver", "hello", "3", "1"]);
}

#[test]
fn test_start_off_end() {
    compare(&["driver", "hi", "5"]);
}

#[test]
fn test_stop_off_end() {
    compare(&["driver", "hi", "0", "5"]);
}

#[test]
fn test_non_integer_start() {
    compare(&["driver", "hello", "abc"]);
}

#[test]
fn test_single_char() {
    compare(&["driver", "hello", "1", "2"]);
}

#[test]
fn test_empty_string() {
    compare(&["driver", ""]);
}

#[test]
fn test_empty_string_with_start() {
    compare(&["driver", "", "0"]);
}

#[test]
fn test_negative_start() {
    compare(&["driver", "hello", "-1"]);
}

#[test]
fn test_negative_stop() {
    compare(&["driver", "hello", "0", "-1"]);
}
