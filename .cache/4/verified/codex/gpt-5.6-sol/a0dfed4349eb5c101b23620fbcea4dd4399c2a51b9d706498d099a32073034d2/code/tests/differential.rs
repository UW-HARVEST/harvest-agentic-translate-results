use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

type Driver = unsafe extern "C" fn(c_int);
type Main = unsafe extern "C" fn() -> c_int;

const STDIN_FILENO: c_int = 0;
const STDOUT_FILENO: c_int = 1;

unsafe extern "C" {
    static mut stdin: *mut c_void;

    fn __fpurge(stream: *mut c_void);
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn compile_c_library(manifest_dir: &Path) -> PathBuf {
    let output_dir = manifest_dir.join("c_src/build");
    let output = output_dir.join("libdriver_c.so");
    fs::create_dir_all(&output_dir).expect("create C build directory");

    let status = Command::new("cc")
        .args(["-fPIC", "-shared", "-o"])
        .arg(&output)
        .arg(manifest_dir.join("c_src/src/main.c"))
        .status()
        .expect("run C compiler");
    assert!(status.success(), "C shared-library compilation failed");
    output
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("locate test executable");
    let deps_dir = test_executable.parent().expect("test deps directory");
    let candidates = [
        deps_dir.join("libdriver.so"),
        deps_dir
            .parent()
            .expect("target profile directory")
            .join("libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("Rust cdylib not found beside {}", test_executable.display()))
}

unsafe fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }

    let mut output_pipe = [-1; 2];
    unsafe {
        assert_eq!(pipe(output_pipe.as_mut_ptr()), 0);
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    unsafe {
        assert_eq!(dup2(output_pipe[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(output_pipe[1]), 0);
    }

    let result = operation();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(output_pipe[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
    }
    (result, output)
}

unsafe fn call_driver(function: &Symbol<'_, Driver>, value: c_int) -> Vec<u8> {
    unsafe { capture_stdout(|| function(value)).1 }
}

unsafe fn call_main(function: &Symbol<'_, Main>, input: &[u8]) -> (c_int, Vec<u8>) {
    let mut input_pipe = [-1; 2];
    unsafe {
        assert_eq!(pipe(input_pipe.as_mut_ptr()), 0);
    }
    File::from(unsafe { std::os::fd::OwnedFd::from_raw_fd(input_pipe[1]) })
        .write_all(input)
        .expect("write simulated stdin");

    let saved_stdin = unsafe { dup(STDIN_FILENO) };
    assert!(saved_stdin >= 0);
    unsafe {
        __fpurge(stdin);
        clearerr(stdin);
        assert_eq!(dup2(input_pipe[0], STDIN_FILENO), STDIN_FILENO);
        assert_eq!(close(input_pipe[0]), 0);
    }

    let result = unsafe { capture_stdout(|| function()) };

    unsafe {
        __fpurge(stdin);
        clearerr(stdin);
        assert_eq!(dup2(saved_stdin, STDIN_FILENO), STDIN_FILENO);
        assert_eq!(close(saved_stdin), 0);
        clearerr(stdin);
    }
    result
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

#[test]
fn all_c_and_rust_ffi_paths_match() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = compile_c_library(&manifest_dir);
    let rust_path = rust_library_path();

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("load C driver");
    let rust_driver: Symbol<'_, Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver");
    let c_main: Symbol<'_, Main> = unsafe { c_library.get(b"main\0") }.expect("load C main");
    let rust_main: Symbol<'_, Main> =
        unsafe { rust_library.get(b"main\0") }.expect("load Rust main");

    let mut driver_values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    let mut random_state = 0x5eed_c0de_d15c_a11du64;
    for _ in 0..1024 {
        driver_values.push(next_random(&mut random_state) as c_int);
    }

    for value in driver_values {
        let c_output = unsafe { call_driver(&c_driver, value) };
        let rust_output = unsafe { call_driver(&rust_driver, value) };
        assert_eq!(rust_output, c_output, "driver diverged for {value}");
    }

    for index in 0..256 {
        let value = next_random(&mut random_state) as c_int;
        let input = match index % 3 {
            0 => format!("{value}\n"),
            1 => format!("  {value}\t"),
            _ => format!("{value:+}\n"),
        };
        let c_result = unsafe { call_main(&c_main, input.as_bytes()) };
        let rust_result = unsafe { call_main(&rust_main, input.as_bytes()) };
        assert_eq!(rust_result, c_result, "main diverged for {input:?}");
    }

    for input in [
        b"".as_slice(),
        b"x\n",
        b" \t\n",
        b"+\n",
        b"2147483648\n",
        b"-2147483649\n",
    ] {
        let c_result = unsafe { call_main(&c_main, input) };
        let rust_result = unsafe { call_main(&rust_main, input) };
        assert_eq!(
            rust_result, c_result,
            "main boundary behavior diverged for {input:?}"
        );
    }
}
