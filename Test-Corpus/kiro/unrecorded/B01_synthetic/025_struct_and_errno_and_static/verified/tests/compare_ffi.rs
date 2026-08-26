use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built alongside the test artifacts
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

/// Capture stdout while calling `f`, return what was printed.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Create a pipe
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let (read_fd, write_fd) = (fds[0], fds[1]);

    // Save original stdout
    let orig_stdout = unsafe { libc::dup(1) };

    // Redirect stdout to write end of pipe
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    // Flush C stdout before calling
    unsafe { libc::fflush(std::ptr::null_mut()) };

    f();

    // Flush after calling
    unsafe { libc::fflush(std::ptr::null_mut()) };
    // Also flush Rust's stdout
    use std::io::Write;
    std::io::stdout().flush().ok();

    // Restore original stdout
    unsafe { libc::dup2(orig_stdout, 1) };
    unsafe { libc::close(orig_stdout) };

    // Read captured output
    let mut buf = String::new();
    let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    read_file.read_to_string(&mut buf).unwrap();
    buf
}

/// Test `run` with a given input, comparing C and Rust outputs.
/// Each call is done in a subprocess to get clean global state.
fn compare_run_in_subprocess(extra_bedrooms: i32) -> (String, String) {
    // We need subprocesses because both libraries have mutable global state
    // that persists across calls. A fresh dlopen each time in a child process
    // gives us clean state.

    let c_output = run_in_child("c", extra_bedrooms);
    let rust_output = run_in_child("rust", extra_bedrooms);
    (c_output, rust_output)
}

fn run_in_child(which: &str, extra_bedrooms: i32) -> String {
    use std::io::Write;

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let (read_fd, write_fd) = (fds[0], fds[1]);

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Child process
        unsafe { libc::close(read_fd) };
        unsafe { libc::dup2(write_fd, 1) }; // redirect stdout to pipe
        unsafe { libc::close(write_fd) };

        let lib_path = if which == "c" { c_lib_path() } else { rust_lib_path() };
        unsafe {
            let lib = Library::new(&lib_path).expect("failed to load lib");
            let run_fn: Symbol<unsafe extern "C" fn(i32)> =
                lib.get(b"run").expect("failed to find run");
            run_fn(extra_bedrooms);
            libc::fflush(std::ptr::null_mut());
        }
        std::io::stdout().flush().ok();
        unsafe { libc::_exit(0) };
    }

    // Parent
    unsafe { libc::close(write_fd) };
    let mut buf = String::new();
    let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    read_file.read_to_string(&mut buf).unwrap();

    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    assert!(
        unsafe { libc::WIFEXITED(status) } && unsafe { libc::WEXITSTATUS(status) } == 0,
        "{which} child exited with non-zero status"
    );
    buf
}

/// Compare `main` function: feed input via stdin pipe, capture stdout.
fn compare_main_in_subprocess(input: &str) -> (String, String) {
    let c_output = main_in_child("c", input);
    let rust_output = main_in_child("rust", input);
    (c_output, rust_output)
}

fn main_in_child(which: &str, input: &str) -> String {
    use std::io::Write;

    // Pipe for stdout capture
    let mut out_fds = [0i32; 2];
    unsafe { libc::pipe(out_fds.as_mut_ptr()) };
    let (out_read, out_write) = (out_fds[0], out_fds[1]);

    // Pipe for stdin
    let mut in_fds = [0i32; 2];
    unsafe { libc::pipe(in_fds.as_mut_ptr()) };
    let (in_read, in_write) = (in_fds[0], in_fds[1]);

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Child
        unsafe {
            libc::close(out_read);
            libc::close(in_write);
            libc::dup2(out_write, 1);
            libc::close(out_write);
            libc::dup2(in_read, 0);
            libc::close(in_read);
        }

        let lib_path = if which == "c" { c_lib_path() } else { rust_lib_path() };
        unsafe {
            let lib = Library::new(&lib_path).expect("failed to load lib");
            let main_fn: Symbol<unsafe extern "C" fn() -> i32> =
                lib.get(b"main").expect("failed to find main");
            main_fn();
            libc::fflush(std::ptr::null_mut());
        }
        std::io::stdout().flush().ok();
        unsafe { libc::_exit(0) };
    }

    // Parent
    unsafe {
        libc::close(out_write);
        libc::close(in_read);
    }

    // Write input to child's stdin
    {
        let mut in_file = unsafe { std::fs::File::from_raw_fd(in_write) };
        in_file.write_all(input.as_bytes()).ok();
        // in_file drops here, closing the write end
    }

    let mut buf = String::new();
    let mut out_file = unsafe { std::fs::File::from_raw_fd(out_read) };
    out_file.read_to_string(&mut buf).unwrap();

    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    assert!(
        unsafe { libc::WIFEXITED(status) } && unsafe { libc::WEXITSTATUS(status) } == 0,
        "{which} child exited with non-zero status"
    );
    buf
}

#[test]
fn test_run_positive() {
    let (c, r) = compare_run_in_subprocess(3);
    assert_eq!(c, r, "run(3) output mismatch");
}

#[test]
fn test_run_zero() {
    let (c, r) = compare_run_in_subprocess(0);
    assert_eq!(c, r, "run(0) output mismatch");
}

#[test]
fn test_run_negative() {
    let (c, r) = compare_run_in_subprocess(-2);
    assert_eq!(c, r, "run(-2) output mismatch");
}

#[test]
fn test_main_valid_input() {
    let (c, r) = compare_main_in_subprocess("3\n");
    assert_eq!(c, r, "main with input '3' mismatch");
}

#[test]
fn test_main_invalid_input() {
    let (c, r) = compare_main_in_subprocess("abc\n");
    assert_eq!(c, r, "main with input 'abc' mismatch");
}

#[test]
fn test_main_negative_input() {
    let (c, r) = compare_main_in_subprocess("-5\n");
    assert_eq!(c, r, "main with input '-5' mismatch");
}

#[test]
fn test_main_zero_input() {
    let (c, r) = compare_main_in_subprocess("0\n");
    assert_eq!(c, r, "main with input '0' mismatch");
}

#[test]
fn test_main_empty_input() {
    let (c, r) = compare_main_in_subprocess("\n");
    assert_eq!(c, r, "main with empty input mismatch");
}
