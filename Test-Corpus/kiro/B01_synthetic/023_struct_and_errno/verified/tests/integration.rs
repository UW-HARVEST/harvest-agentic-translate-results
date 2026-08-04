use libloading::{Library, Symbol};
use std::process::{Command, Stdio};
use std::io::Write;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct house_t {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver_c.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

fn capture_run(lib_path: &str, house: house_t, extra_bedrooms: i32) -> (String, house_t) {
    // We need to capture stdout. Since both libs use printf/print! which write to fd 1,
    // we'll fork a child process that loads the lib and calls run, capturing its stdout.
    // But for simplicity, use gag crate or pipe. Let's use a helper binary approach.
    // Actually, the simplest: load the lib, redirect stdout via pipe, call run.
    
    // For C and Rust .so, `run` writes to stdout via printf / print!.
    // We'll capture by using a pipe on fd 1.
    let mut pipe_fds = [0i32; 2];
    unsafe { libc_pipe(&mut pipe_fds) };
    
    let old_stdout = unsafe { libc_dup(1) };
    unsafe { libc_dup2(pipe_fds[1], 1) };
    unsafe { libc_close(pipe_fds[1]) };
    
    let mut h = house;
    unsafe {
        let lib = Library::new(lib_path).expect("Failed to load library");
        let func: Symbol<unsafe extern "C" fn(*mut house_t, i32)> =
            lib.get(b"run").expect("Failed to find run");
        func(&mut h as *mut house_t, extra_bedrooms);
        // flush C stdout
        libc_fflush(std::ptr::null_mut());
    }
    
    unsafe { libc_dup2(old_stdout, 1) };
    unsafe { libc_close(old_stdout) };
    
    // Read from pipe
    let mut buf = vec![0u8; 4096];
    let n = unsafe { libc_read(pipe_fds[0], buf.as_mut_ptr() as *mut _, buf.len()) };
    unsafe { libc_close(pipe_fds[0]) };
    
    let output = String::from_utf8_lossy(&buf[..n as usize]).to_string();
    (output, h)
}

extern "C" {
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn read(fd: i32, buf: *mut std::ffi::c_void, count: usize) -> isize;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

unsafe fn libc_pipe(fds: &mut [i32; 2]) -> i32 { pipe(fds.as_mut_ptr()) }
unsafe fn libc_dup(fd: i32) -> i32 { dup(fd) }
unsafe fn libc_dup2(old: i32, new: i32) -> i32 { dup2(old, new) }
unsafe fn libc_close(fd: i32) -> i32 { close(fd) }
unsafe fn libc_read(fd: i32, buf: *mut u8, count: usize) -> isize { read(fd, buf as *mut _, count) }
unsafe fn libc_fflush(stream: *mut std::ffi::c_void) -> i32 { fflush(stream) }

#[test]
fn test_run_basic() {
    let house = house_t { floors: 2, bedrooms: 5, bathrooms: 2.5 };
    let (c_out, c_house) = capture_run(C_LIB, house, 3);
    let (rs_out, rs_house) = capture_run(RUST_LIB, house, 3);
    assert_eq!(c_out, rs_out, "run output mismatch for input 3");
    assert_eq!(c_house, rs_house, "run house state mismatch for input 3");
}

#[test]
fn test_run_zero() {
    let house = house_t { floors: 2, bedrooms: 5, bathrooms: 2.5 };
    let (c_out, c_house) = capture_run(C_LIB, house, 0);
    let (rs_out, rs_house) = capture_run(RUST_LIB, house, 0);
    assert_eq!(c_out, rs_out, "run output mismatch for input 0");
    assert_eq!(c_house, rs_house, "run house state mismatch for input 0");
}

#[test]
fn test_run_negative() {
    let house = house_t { floors: 2, bedrooms: 5, bathrooms: 2.5 };
    let (c_out, c_house) = capture_run(C_LIB, house, -2);
    let (rs_out, rs_house) = capture_run(RUST_LIB, house, -2);
    assert_eq!(c_out, rs_out, "run output mismatch for input -2");
    assert_eq!(c_house, rs_house, "run house state mismatch for input -2");
}

fn run_main_with_input(lib_so: &str, input: &str) -> String {
    // Build a small helper: use the executable built from the .so's main
    // Actually, we can't easily call main from .so and capture stdin+stdout.
    // Instead, build both as executables and pipe input.
    // C executable is at c_src/build/driver, Rust at target/debug/driver
    let exe = if lib_so == C_LIB {
        concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver")
    } else {
        concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/driver")
    };
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_main_valid_input() {
    let c_out = run_main_with_input(C_LIB, "3\n");
    let rs_out = run_main_with_input(RUST_LIB, "3\n");
    assert_eq!(c_out, rs_out, "main output mismatch for '3'");
}

#[test]
fn test_main_invalid_input() {
    let c_out = run_main_with_input(C_LIB, "abc\n");
    let rs_out = run_main_with_input(RUST_LIB, "abc\n");
    assert_eq!(c_out, rs_out, "main output mismatch for 'abc'");
}

#[test]
fn test_main_negative_input() {
    let c_out = run_main_with_input(C_LIB, "-5\n");
    let rs_out = run_main_with_input(RUST_LIB, "-5\n");
    assert_eq!(c_out, rs_out, "main output mismatch for '-5'");
}

#[test]
fn test_main_zero_input() {
    let c_out = run_main_with_input(C_LIB, "0\n");
    let rs_out = run_main_with_input(RUST_LIB, "0\n");
    assert_eq!(c_out, rs_out, "main output mismatch for '0'");
}

#[test]
fn test_main_empty_input() {
    let c_out = run_main_with_input(C_LIB, "\n");
    let rs_out = run_main_with_input(RUST_LIB, "\n");
    assert_eq!(c_out, rs_out, "main output mismatch for empty");
}
