use libloading::{Library, Symbol};
use std::ffi::{c_char, CString};
use std::io::Read;
use std::os::fd::FromRawFd;

const C_LIB: &str = "c_src/build/libdriver.so";
const RUST_LIB: &str = "target/debug/libdriver.so";

unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn close(fd: i32) -> i32;
}

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        let stdout_fd: i32 = 1;
        libc::fflush(std::ptr::null_mut());

        let saved = dup(stdout_fd);
        assert!(saved >= 0);

        let mut fds = [0i32; 2];
        assert!(pipe(fds.as_mut_ptr()) == 0);

        assert!(dup2(fds[1], stdout_fd) >= 0);
        close(fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());

        dup2(saved, stdout_fd);
        close(saved);

        let mut output = Vec::new();
        let mut file = std::fs::File::from_raw_fd(fds[0]);
        let _ = file.read_to_end(&mut output);
        output
    }
}

fn run_no_arg(lib_path: &str, sym_name: &[u8]) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load lib");
        let f: Symbol<unsafe extern "C" fn()> = lib.get(sym_name).expect("symbol");
        capture_stdout(|| f())
    }
}

fn run_print_line(lib_path: &str, line: Option<&str>) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load lib");
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine").expect("symbol");
        let cstr = line.map(|s| CString::new(s).unwrap());
        let ptr = cstr
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());
        capture_stdout(|| f(ptr))
    }
}

#[test]
fn test_print_line_null() {
    let c_out = run_print_line(C_LIB, None);
    let r_out = run_print_line(RUST_LIB, None);
    assert_eq!(c_out, r_out, "printLine(NULL) mismatch");
}

#[test]
fn test_print_line_empty() {
    let c_out = run_print_line(C_LIB, Some(""));
    let r_out = run_print_line(RUST_LIB, Some(""));
    assert_eq!(c_out, r_out, "printLine(\"\") mismatch");
}

#[test]
fn test_print_line_simple() {
    let c_out = run_print_line(C_LIB, Some("hello world"));
    let r_out = run_print_line(RUST_LIB, Some("hello world"));
    assert_eq!(c_out, r_out, "printLine simple mismatch");
}

#[test]
fn test_print_line_special() {
    let c_out = run_print_line(C_LIB, Some("line with\ttabs and \"quotes\""));
    let r_out = run_print_line(RUST_LIB, Some("line with\ttabs and \"quotes\""));
    assert_eq!(c_out, r_out, "printLine special mismatch");
}

#[test]
fn test_bad() {
    let c_out = run_no_arg(C_LIB, b"bad");
    let r_out = run_no_arg(RUST_LIB, b"bad");
    assert_eq!(c_out, r_out, "bad() mismatch");
}

#[test]
fn test_good() {
    let c_out = run_no_arg(C_LIB, b"good");
    let r_out = run_no_arg(RUST_LIB, b"good");
    assert_eq!(c_out, r_out, "good() mismatch");
}

#[test]
fn test_driver() {
    let c_out = run_no_arg(C_LIB, b"driver");
    let r_out = run_no_arg(RUST_LIB, b"driver");
    assert_eq!(c_out, r_out, "driver() mismatch");
}
