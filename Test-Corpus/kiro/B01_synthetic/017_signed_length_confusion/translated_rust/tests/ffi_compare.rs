use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libtranslated_rust.so")
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::os::unix::io::FromRawFd;
    use std::io::Read;

    // Flush any pending stdout
    unsafe { libc::fflush(std::ptr::null_mut()); }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }

    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    // Flush C and Rust stdout
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe { libc::dup2(old_stdout, 1); }
    unsafe {
        libc::close(old_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    // Set non-blocking to avoid hanging
    unsafe {
        let flags = libc::fcntl(pipe_fds[0], libc::F_GETFL);
        libc::fcntl(pipe_fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf);
    buf
}

#[test]
fn test_printline_normal_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            rust_lib.get(b"printLine").expect("Rust printLine");

        let test_str = CString::new("Hello, World!").unwrap();

        let c_out = capture_stdout(|| { c_fn(test_str.as_ptr()); });
        let r_out = capture_stdout(|| { r_fn(test_str.as_ptr()); });

        assert_eq!(c_out, r_out, "printLine normal string mismatch:\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}

#[test]
fn test_printline_empty_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            rust_lib.get(b"printLine").expect("Rust printLine");

        let test_str = CString::new("").unwrap();

        let c_out = capture_stdout(|| { c_fn(test_str.as_ptr()); });
        let r_out = capture_stdout(|| { r_fn(test_str.as_ptr()); });

        assert_eq!(c_out, r_out, "printLine empty string mismatch");
    }
}

#[test]
fn test_printline_null() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            rust_lib.get(b"printLine").expect("Rust printLine");

        let c_out = capture_stdout(|| { c_fn(std::ptr::null()); });
        let r_out = capture_stdout(|| { r_fn(std::ptr::null()); });

        assert_eq!(c_out, r_out, "printLine null mismatch");
    }
}

#[test]
fn test_printline_special_chars() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").expect("C printLine");
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
            rust_lib.get(b"printLine").expect("Rust printLine");

        for s in &["tab\there", "new\nline", "  spaces  ", "AAAAAAAAAA"] {
            let test_str = CString::new(*s).unwrap();
            let c_out = capture_stdout(|| { c_fn(test_str.as_ptr()); });
            let r_out = capture_stdout(|| { r_fn(test_str.as_ptr()); });
            assert_eq!(c_out, r_out, "printLine mismatch for {:?}", s);
        }
    }
}

/// Test main() by comparing executable outputs (since main reads stdin).
/// We build both C and Rust executables and pipe the same input.
#[test]
fn test_main_via_executables() {
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Build C executable
    let c_exe = manifest_dir.join("c_src/build/driver");
    if !c_exe.exists() {
        let build_dir = manifest_dir.join("c_src/build");
        std::fs::create_dir_all(&build_dir).unwrap();
        let status = Command::new("cmake")
            .args(&["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .current_dir(&build_dir)
            .status().unwrap();
        assert!(status.success(), "cmake configure failed");
        let status = Command::new("cmake")
            .args(&["--build", "."])
            .current_dir(&build_dir)
            .status().unwrap();
        assert!(status.success(), "cmake build failed");
    }

    // Build Rust executable
    let status = Command::new("cargo")
        .args(&["build", "--bin", "driver"])
        .current_dir(&manifest_dir)
        .status().unwrap();
    assert!(status.success(), "cargo build failed");
    let rust_exe = manifest_dir.join("target/debug/driver");

    // Test inputs: valid numbers within range
    for input in &["5\n", "0\n", "50\n", "99\n"] {
        let c_output = Command::new(&c_exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            }).unwrap();

        let r_output = Command::new(&rust_exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            }).unwrap();

        assert_eq!(c_output.stdout, r_output.stdout,
            "main stdout mismatch for input {:?}:\nC:    {:?}\nRust: {:?}",
            input, String::from_utf8_lossy(&c_output.stdout),
            String::from_utf8_lossy(&r_output.stdout));
    }
}

/// Test main with input >= 100 (the if branch is skipped, empty dest printed)
#[test]
fn test_main_large_input() {
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_exe = manifest_dir.join("c_src/build/driver");
    let rust_exe = manifest_dir.join("target/debug/driver");

    for input in &["100\n", "200\n"] {
        let c_output = Command::new(&c_exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            }).unwrap();

        let r_output = Command::new(&rust_exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            }).unwrap();

        assert_eq!(c_output.stdout, r_output.stdout,
            "main stdout mismatch for input {:?}:\nC:    {:?}\nRust: {:?}",
            input, String::from_utf8_lossy(&c_output.stdout),
            String::from_utf8_lossy(&r_output.stdout));
    }
}

/// Test main with EOF on stdin (no input at all)
#[test]
fn test_main_eof_stdin() {
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_exe = manifest_dir.join("c_src/build/driver");
    let rust_exe = manifest_dir.join("target/debug/driver");

    // Empty stdin - triggers "fgets() failed." path
    let c_output = Command::new(&c_exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take()); // close stdin immediately
            child.wait_with_output()
        }).unwrap();

    let r_output = Command::new(&rust_exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            drop(child.stdin.take());
            child.wait_with_output()
        }).unwrap();

    assert_eq!(c_output.stdout, r_output.stdout,
        "main EOF stdin mismatch:\nC:    {:?}\nRust: {:?}",
        String::from_utf8_lossy(&c_output.stdout),
        String::from_utf8_lossy(&r_output.stdout));
}
