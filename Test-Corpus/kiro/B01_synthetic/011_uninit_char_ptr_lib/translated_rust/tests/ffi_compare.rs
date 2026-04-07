use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in deps/ during test builds
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    // Try direct path first, then deps/
    let direct = dir.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    // Search deps/ for the cdylib
    for entry in std::fs::read_dir(dir.join("deps")).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libdriver") && name.ends_with(".so") && !name.contains(".d") {
                return p;
            }
        }
    }
    panic!("Could not find Rust libdriver.so");
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    // Flush Rust stdout first
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()) };
    let (read_fd, write_fd) = (pipes[0], pipes[1]);

    let saved = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(saved, 1) };
    unsafe { libc::close(saved) };

    let mut buf = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_end(&mut buf).unwrap();
    buf
}

#[test]
fn test_print_line_with_string() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = CString::new("hello world").unwrap();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(input.as_ptr());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(input.as_ptr());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for normal string");
}

#[test]
fn test_print_line_with_null() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(std::ptr::null());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for NULL");
}

#[test]
fn test_print_line_empty_string() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let input = CString::new("").unwrap();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = c_lib.get(b"printLine").unwrap();
        f(input.as_ptr());
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = r_lib.get(b"printLine").unwrap();
        f(input.as_ptr());
    });
    assert_eq!(c_out, r_out, "printLine mismatch for empty string");
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });
    assert_eq!(c_out, r_out, "good() output mismatch");
    assert_eq!(c_out, b"string\n", "good() should print 'string\\n'");
}

#[test]
fn test_driver_good_path() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"driver").unwrap();
        f(1);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"driver").unwrap();
        f(1);
    });
    assert_eq!(c_out, r_out, "driver(1) output mismatch");
    assert_eq!(c_out, b"string\n", "driver(1) should print 'string\\n'");
}

#[test]
fn test_driver_various_nonzero() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    for val in &[1i32, 2, -1, 100, i32::MAX] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"driver").unwrap();
            f(*val);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"driver").unwrap();
            f(*val);
        });
        assert_eq!(c_out, r_out, "driver({}) output mismatch", val);
    }
}
