use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::raw::{c_char, c_int};
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_lib_build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cdylib is always built in deps or directly in debug/
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libdriver.so")
}

/// Capture stdout (fd 1) during a closure by redirecting it to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {

    // Flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    // Flush C and Rust stdout
    unsafe { libc::fflush(std::ptr::null_mut()) };
    // Also flush Rust's stdout
    let _ = std::io::stdout().flush();

    // Restore stdout
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };

    // Read captured output
    let mut output = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(read_fd) };
    reader.read_to_string(&mut output).unwrap();
    output
}

/// Capture stdout while also providing input on stdin via a pipe.
fn capture_stdout_with_stdin<F: FnOnce()>(input: &str, f: F) -> String {
    // Flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    // Set up stdin pipe
    let mut stdin_pipe = [0i32; 2];
    unsafe { libc::pipe(stdin_pipe.as_mut_ptr()) };
    let stdin_read = stdin_pipe[0];
    let stdin_write = stdin_pipe[1];

    // Write input data
    let mut writer = unsafe { std::fs::File::from_raw_fd(stdin_write) };
    writer.write_all(input.as_bytes()).unwrap();
    drop(writer); // close write end so reader gets EOF

    let saved_stdin = unsafe { libc::dup(0) };
    unsafe { libc::dup2(stdin_read, 0) };
    unsafe { libc::close(stdin_read) };

    // Set up stdout pipe
    let mut stdout_pipe = [0i32; 2];
    unsafe { libc::pipe(stdout_pipe.as_mut_ptr()) };
    let stdout_read = stdout_pipe[0];
    let stdout_write = stdout_pipe[1];

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(stdout_write, 1) };
    unsafe { libc::close(stdout_write) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    // Restore
    unsafe { libc::dup2(saved_stdout, 1) };
    unsafe { libc::close(saved_stdout) };
    unsafe { libc::dup2(saved_stdin, 0) };
    unsafe { libc::close(saved_stdin) };

    let mut output = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(stdout_read) };
    reader.read_to_string(&mut output).unwrap();
    output
}

#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    for val in [0, 1, -1, 42, i32::MAX, i32::MIN, 7, 999] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"printIntLine").unwrap();
            f(val);
        });
        assert_eq!(c_out, r_out, "printIntLine mismatch for input {}", val);
    }
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let cases = ["hello", "", "test with spaces", "ERROR: Array index is negative."];
    for s in cases {
        let cs = CString::new(s).unwrap();
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
            f(cs.as_ptr());
        });
        assert_eq!(c_out, r_out, "printLine mismatch for input {:?}", s);
    }

    // Test NULL
    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for NULL input");
}

#[test]
fn test_good_with_valid_index() {
    // good() calls goodG2B() which uses data=7 (deterministic),
    // then goodB2G() which reads from stdin.
    // Provide a valid index (e.g., "3\n") for goodB2G.
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = "3\n";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() mismatch with stdin='3\\n'");
}

#[test]
fn test_good_with_negative_index() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = "-1\n";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() mismatch with stdin='-1\\n'");
}

#[test]
fn test_good_with_out_of_bounds_index() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = "10\n";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() mismatch with stdin='10\\n'");
}

#[test]
fn test_bad_with_valid_index() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Use index within bounds to avoid UB
    let input = "5\n";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() mismatch with stdin='5\\n'");
}

#[test]
fn test_bad_with_negative_index() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = "-5\n";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() mismatch with stdin='-5\\n'");
}

#[test]
fn test_bad_eof_stdin() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Empty stdin -> fgets returns NULL
    let input = "";
    let c_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"bad").unwrap();
        f();
    });
    let r_out = capture_stdout_with_stdin(input, || unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"bad").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "bad() mismatch with empty stdin (EOF)");
}
