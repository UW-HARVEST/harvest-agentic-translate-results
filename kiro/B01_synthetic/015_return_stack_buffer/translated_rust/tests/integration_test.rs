use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::process::Command;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    // Use a pipe via a child process approach won't work for in-process calls.
    // Instead, redirect stdout using glibc tricks.
    use std::fs;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        // Also flush Rust's stdout
        use std::io::Write;
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());

        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        let mut reader = fs::File::from_raw_fd(pipe_fds[0]);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        buf
    }
}

/// Test printLine: lowest-level function
#[test]
fn test_printline() {
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_printLine: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("Failed to find printLine");

        let test_str = CString::new("hello world").unwrap();

        let c_output = capture_stdout(|| {
            c_printLine(test_str.as_ptr());
        });

        let rust_output = capture_stdout(|| {
            driver::printLine(test_str.as_ptr());
        });

        assert_eq!(c_output, rust_output, "printLine mismatch:\n  C: {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_output), String::from_utf8_lossy(&rust_output));
    }
}

/// Test printLine with NULL
#[test]
fn test_printline_null() {
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_printLine: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("Failed to find printLine");

        let c_output = capture_stdout(|| {
            c_printLine(std::ptr::null());
        });

        let rust_output = capture_stdout(|| {
            driver::printLine(std::ptr::null());
        });

        assert_eq!(c_output, rust_output, "printLine(NULL) mismatch");
    }
}

/// Test good(): calls helperGood1 (static local) then printLine
#[test]
fn test_good() {
    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_good: Symbol<unsafe extern "C" fn()> =
            c_lib.get(b"good").expect("Failed to find good");

        let c_output = capture_stdout(|| {
            c_good();
        });

        let rust_output = capture_stdout(|| {
            driver::good();
        });

        assert_eq!(c_output, rust_output, "good() mismatch:\n  C: {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_output), String::from_utf8_lossy(&rust_output));
    }
}

/// Test binary output: echo "1" | driver should produce same output
#[test]
fn test_binary_good_path() {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let c_bin = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");

    let c_out = Command::new(c_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"1\n").unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run C binary");

    let rust_out = Command::new(rust_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"1\n").unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run Rust binary");

    assert_eq!(c_out.stdout, rust_out.stdout,
        "Binary output mismatch (input=1):\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out.stdout), String::from_utf8_lossy(&rust_out.stdout));
}
