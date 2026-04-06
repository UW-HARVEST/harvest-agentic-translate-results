use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/target/debug/libdriver.so", manifest)
}

/// Capture stdout by redirecting fd 1 to a temp file.
/// Works for C functions that use printf (C stdio).
/// For Rust .so functions that use println!, we need special handling.
fn capture_c_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut());

        let tmpfile = libc::tmpfile();
        let tmp_fd = libc::fileno(tmpfile);

        let saved_stdout = libc::dup(1);
        libc::dup2(tmp_fd, 1);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        libc::lseek(tmp_fd, 0, libc::SEEK_SET);
        let mut reader = std::fs::File::from_raw_fd(tmp_fd);
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    }
}

/// For Rust .so functions, we also need to redirect fd 1 but the Rust
/// println! macro writes through Rust's stdout which wraps fd 1.
/// The key is to make sure Rust's stdout LineWriter flushes after each line.
/// println! should auto-flush since LineWriter flushes on newline.
/// We just need to make sure the fd redirect happens before any Rust stdout lock.
fn capture_rust_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::{Read, Write};
    use std::os::unix::io::FromRawFd;

    unsafe {
        // Flush everything first
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());

        let tmpfile = libc::tmpfile();
        let tmp_fd = libc::fileno(tmpfile);

        let saved_stdout = libc::dup(1);
        libc::dup2(tmp_fd, 1);

        f();

        // Flush Rust stdout to ensure data hits fd 1
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());

        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        libc::lseek(tmp_fd, 0, libc::SEEK_SET);
        let mut reader = std::fs::File::from_raw_fd(tmp_fd);
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    }
}

#[test]
fn test_print_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rust_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { rust_lib.get(b"printLine").unwrap() };

    for s in ["hello world", "", "test 123"] {
        let cs = CString::new(s).unwrap();
        let c_out = capture_c_stdout(|| unsafe { c_fn(cs.as_ptr()) });
        let r_out = capture_rust_stdout(|| unsafe { r_fn(cs.as_ptr()) });
        assert_eq!(c_out, r_out, "printLine mismatch for {:?}", s);
    }

    // NULL test
    let c_out = capture_c_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture_rust_stdout(|| unsafe { r_fn(std::ptr::null()) });
    assert_eq!(c_out, r_out, "printLine mismatch for NULL");
}

#[test]
fn test_print_int_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rust_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { c_lib.get(b"printIntLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { rust_lib.get(b"printIntLine").unwrap() };

    for val in [0, 1, -1, 42, i32::MAX, i32::MIN, 7, 9, 10] {
        let c_out = capture_c_stdout(|| unsafe { c_fn(val) });
        let r_out = capture_rust_stdout(|| unsafe { r_fn(val) });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {}", val);
    }
}

/// Test main binary output: pipe same stdin to both C and Rust binaries.
#[test]
fn test_binary_output() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let c_bin = format!("{}/c_src/build/driver", manifest);
    let rust_bin = format!("{}/target/debug/driver", manifest);

    // Ensure Rust binary is built
    let build = std::process::Command::new("cargo")
        .args(["build", "--bin", "driver"])
        .current_dir(&manifest)
        .output()
        .expect("cargo build failed");
    assert!(build.status.success(), "cargo build failed: {}", String::from_utf8_lossy(&build.stderr));

    // Input: "5\n" for goodB2G (inside good()), then "5\n" for bad()
    let input = b"5\n5\n";

    let run = |bin: &str| -> Vec<u8> {
        use std::io::Write;
        let mut child = std::process::Command::new(bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to run {}: {}", bin, e));
        child.stdin.as_mut().unwrap().write_all(input).unwrap();
        drop(child.stdin.take());
        let out = child.wait_with_output().unwrap();
        out.stdout
    };

    let c_out = run(&c_bin);
    let r_out = run(&rust_bin);

    assert_eq!(
        c_out, r_out,
        "Binary stdout mismatch!\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}
