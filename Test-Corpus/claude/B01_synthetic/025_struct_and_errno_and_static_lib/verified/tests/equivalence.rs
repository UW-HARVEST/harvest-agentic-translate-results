// Integration tests that load both the C and Rust shared libraries via
// libloading and compare their outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

const C_LIB_PATH: &str = "c_src/build/libdriver.so";
const RUST_LIB_PATH: &str = "target/debug/libdriver.so";

/// Capture everything written to stdout while `f` runs and return it as a Vec<u8>.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush any prior libc buffering first.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Save original fd 1.
    let saved_stdout: RawFd = unsafe { libc::dup(1) };
    assert!(saved_stdout >= 0, "dup failed");

    // Create a pipe.
    let mut pipefd: [c_int; 2] = [0; 2];
    let r = unsafe { libc::pipe(pipefd.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe failed");
    let read_fd = pipefd[0];
    let write_fd = pipefd[1];

    // Redirect stdout to pipe write end.
    let r = unsafe { libc::dup2(write_fd, 1) };
    assert!(r >= 0, "dup2 failed");
    unsafe { libc::close(write_fd) };

    // Run the function.
    f();

    // Flush libc-level stdio buffers in this process so output is in the pipe.
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    let r = unsafe { libc::dup2(saved_stdout, 1) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe { libc::close(saved_stdout) };

    // Read all data from pipe.
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = Vec::new();
    // We've closed the write end (the dup'd one inside the process), but fd 1
    // also points to it after dup2 - now restored.  The pipe's write side is
    // gone so read returns EOF.
    file.read_to_end(&mut buf).expect("pipe read");
    buf
}

fn lib_path(rel: &str) -> String {
    // tests run with CWD = the crate root.
    let cwd = std::env::current_dir().expect("cwd");
    cwd.join(rel).to_string_lossy().into_owned()
}

unsafe fn load(lib_rel: &str) -> Library {
    let path = lib_path(lib_rel);
    Library::new(&path).unwrap_or_else(|e| panic!("load {}: {}", path, e))
}

fn run_driver(lib: &Library, input: &str) -> Vec<u8> {
    let cstr = CString::new(input).unwrap();
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"driver\0").expect("driver symbol");
        f(cstr.as_ptr());
    })
}

fn run_run(lib: &Library, n: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"run\0").expect("run symbol");
        f(n);
    })
}

fn show(label: &str, bytes: &[u8]) -> String {
    format!("{}: {:?}", label, String::from_utf8_lossy(bytes))
}

fn assert_eq_bytes(a: &[u8], b: &[u8]) {
    if a != b {
        panic!(
            "Mismatch:\n  {}\n  {}",
            show("C   ", a),
            show("Rust", b)
        );
    }
}

#[test]
fn test_driver_valid_inputs() {
    let inputs = ["0", "1", "-1", "42", "-42", "100", "  7", "+3"];
    for inp in inputs {
        // Use a fresh library load for each input so the global state in each
        // library starts identically.
        let c_lib = unsafe { load(C_LIB_PATH) };
        let rust_lib = unsafe { load(RUST_LIB_PATH) };
        let c_out = run_driver(&c_lib, inp);
        let r_out = run_driver(&rust_lib, inp);
        assert_eq_bytes(&c_out, &r_out);
    }
}

#[test]
fn test_driver_invalid_inputs() {
    let inputs = [
        "",                       // empty -> parse fails (endp == str)
        "abc",                    // no digits
        "9999999999999999999999", // overflow -> errno == ERANGE
        "-9999999999999999999",   // overflow negative
    ];
    for inp in inputs {
        let c_lib = unsafe { load(C_LIB_PATH) };
        let rust_lib = unsafe { load(RUST_LIB_PATH) };
        let c_out = run_driver(&c_lib, inp);
        let r_out = run_driver(&rust_lib, inp);
        assert_eq_bytes(&c_out, &r_out);
    }
}

#[test]
fn test_run_various() {
    for n in [0, 1, -1, 5, -5, 100] {
        let c_lib = unsafe { load(C_LIB_PATH) };
        let rust_lib = unsafe { load(RUST_LIB_PATH) };
        let c_out = run_run(&c_lib, n);
        let r_out = run_run(&rust_lib, n);
        assert_eq_bytes(&c_out, &r_out);
    }
}

#[test]
fn test_run_repeated_state_persists() {
    // Verify state persists across multiple calls in a single library load,
    // and the Rust library matches the C library's state semantics.
    let c_lib = unsafe { load(C_LIB_PATH) };
    let rust_lib = unsafe { load(RUST_LIB_PATH) };

    for n in [3, 7, -2, 0] {
        let c_out = run_run(&c_lib, n);
        let r_out = run_run(&rust_lib, n);
        assert_eq_bytes(&c_out, &r_out);
    }
}

#[test]
fn test_driver_calls_run_twice() {
    // The driver invokes run() twice for the same x; ensure both libraries
    // print the same total output.
    let c_lib = unsafe { load(C_LIB_PATH) };
    let rust_lib = unsafe { load(RUST_LIB_PATH) };

    for inp in ["4", "0", "-3"] {
        let c_out = run_driver(&c_lib, inp);
        let r_out = run_driver(&rust_lib, inp);
        assert_eq_bytes(&c_out, &r_out);
    }
}

// Use libc just for fflush/pipe/dup; pull it in via the driver crate's dep.
extern crate libc;

// Silence unused warnings on non-Unix (this crate targets Linux per the
// build setup).
#[cfg(not(unix))]
compile_error!("tests assume a Unix platform");

// Helpful: ensure AsRawFd / IntoRawFd remain referenced so importing them is
// not flagged in strict configurations.
#[allow(dead_code)]
fn _suppress_unused_imports() {
    let _ = std::io::stdout().as_raw_fd();
    let f: Option<std::fs::File> = None;
    if let Some(f) = f {
        let _ = f.into_raw_fd();
    }
}
