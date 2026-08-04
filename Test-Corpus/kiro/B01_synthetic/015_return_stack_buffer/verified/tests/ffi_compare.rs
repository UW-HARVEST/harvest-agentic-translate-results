use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

/// Capture stdout by redirecting fd 1 to a pipe, calling f(), then restoring.
fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        // Flush before redirect
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc_fflush(std::ptr::null_mut());

        let mut fds = [0i32; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        // Make read end non-blocking isn't needed if we close write end properly

        let saved = dup(1);
        dup2(fds[1], 1);

        f();

        // Flush after call
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc_fflush(std::ptr::null_mut());

        // Restore stdout and close write end
        dup2(saved, 1);
        close(saved);
        close(fds[1]);

        let mut buf = Vec::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_end(&mut buf).unwrap();
        buf
    }
}

extern "C" {
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

unsafe fn libc_fflush(stream: *mut std::ffi::c_void) -> i32 {
    fflush(stream)
}

fn call_print_line(lib: &Library, arg: Option<&str>) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> = lib.get(b"printLine").unwrap();
        match arg {
            Some(s) => {
                let cs = CString::new(s).unwrap();
                f(cs.as_ptr());
            }
            None => f(std::ptr::null()),
        }
    }
}

fn call_void(lib: &Library, name: &[u8]) {
    unsafe {
        let f: Symbol<unsafe extern "C" fn()> = lib.get(name).unwrap();
        f();
    }
}

#[test]
fn test_print_line_null() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_out = capture_stdout(|| call_print_line(&c_lib, None));
    let r_out = capture_stdout(|| call_print_line(&r_lib, None));
    assert_eq!(c_out, r_out, "printLine(NULL): C={:?} Rust={:?}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_print_line_string() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_out = capture_stdout(|| call_print_line(&c_lib, Some("test string")));
    let r_out = capture_stdout(|| call_print_line(&r_lib, Some("test string")));
    assert_eq!(c_out, r_out, "printLine(str): C={:?} Rust={:?}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_print_line_empty() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_out = capture_stdout(|| call_print_line(&c_lib, Some("")));
    let r_out = capture_stdout(|| call_print_line(&r_lib, Some("")));
    assert_eq!(c_out, r_out, "printLine(empty): C={:?} Rust={:?}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_bad() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_out = capture_stdout(|| call_void(&c_lib, b"bad"));
    let r_out = capture_stdout(|| call_void(&r_lib, b"bad"));
    assert_eq!(c_out, r_out, "bad(): C={:?} Rust={:?}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_out = capture_stdout(|| call_void(&c_lib, b"good"));
    let r_out = capture_stdout(|| call_void(&r_lib, b"good"));
    assert_eq!(c_out, r_out, "good(): C={:?} Rust={:?}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}
