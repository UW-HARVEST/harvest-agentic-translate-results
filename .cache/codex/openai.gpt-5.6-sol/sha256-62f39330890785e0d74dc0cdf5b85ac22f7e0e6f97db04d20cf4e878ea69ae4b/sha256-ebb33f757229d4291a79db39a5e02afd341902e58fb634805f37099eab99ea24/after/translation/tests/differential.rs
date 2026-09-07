use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Driver = unsafe extern "C" fn(*const c_char, *const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + (self.next_u64() as usize % (end_inclusive - start + 1))
    }

    fn byte(&mut self, start: u8, end_inclusive: u8) -> u8 {
        start + (self.next_u64() as u8 % (end_inclusive - start + 1))
    }

    fn bytes(&mut self, length: usize, start: u8, end_inclusive: u8) -> Vec<u8> {
        (0..length)
            .map(|_| self.byte(start, end_inclusive))
            .collect()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library must be built before running tests")
}

fn rust_library_path() -> PathBuf {
    let test_binary = std::env::current_exe().expect("test executable path");
    test_binary
        .parent()
        .and_then(Path::parent)
        .expect("Cargo target profile directory")
        .join("libdriver.so")
        .canonicalize()
        .expect("Rust cdylib must be built in the active Cargo profile")
}

unsafe fn load_driver(path: &Path) -> (Library, Driver) {
    let library = unsafe { Library::new(path) }.expect("load shared library");
    let driver = *unsafe { library.get::<Driver>(b"driver\0") }.expect("load driver symbol");
    (library, driver)
}

fn capture_stdout(driver: Driver, s1: *const c_char, s2: *const c_char) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        driver(s1, s2);
        assert_eq!(fflush(std::ptr::null_mut()), 0);

        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 256];
        loop {
            let count = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0);
        output
    }
}

fn compare_case(
    row: usize,
    case_number: usize,
    c_driver: Driver,
    rust_driver: Driver,
    s1: Vec<u8>,
    s2: Vec<u8>,
) {
    let s1 = CString::new(s1).expect("generated s1 has no NUL");
    let s2 = CString::new(s2).expect("generated s2 has no NUL");
    let c_output = capture_stdout(c_driver, s1.as_ptr(), s2.as_ptr());
    let rust_output = capture_stdout(rust_driver, s1.as_ptr(), s2.as_ptr());
    assert_eq!(
        rust_output, c_output,
        "CONFIGS.md row {row}, randomized case {case_number}"
    );
}

#[test]
fn valid_configuration_surface_matches_byte_for_byte() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    let (_c_library, c_driver) = unsafe { load_driver(&c_path) };
    let (_rust_library, rust_driver) = unsafe { load_driver(&rust_path) };
    let mut rng = Rng::new(0x6d5a_56e9_27c4_13ab);

    for case_number in 0..256 {
        let s2 = if case_number % 2 == 0 {
            Vec::new()
        } else {
            let length = rng.usize(1, 128);
            rng.bytes(length, 1, 255)
        };
        compare_case(1, case_number, c_driver, rust_driver, Vec::new(), s2);
    }

    for case_number in 0..256 {
        let length = rng.usize(1, 1024);
        let s1 = rng.bytes(length, 1, 255);
        compare_case(2, case_number, c_driver, rust_driver, s1, Vec::new());
    }

    for case_number in 0..256 {
        let length = rng.usize(1, 1024);
        let hit = rng.byte(1, 255);
        let mut s1 = rng.bytes(length, 1, 255);
        s1[0] = hit;
        let s2_length = rng.usize(1, 128);
        let mut s2 = rng.bytes(s2_length, 1, 255);
        let hit_index = rng.usize(0, s2.len() - 1);
        s2[hit_index] = hit;
        compare_case(3, case_number, c_driver, rust_driver, s1, s2);
    }

    for case_number in 0..256 {
        let prefix_length = rng.usize(1, 512);
        let suffix_length = rng.usize(0, 512);
        let hit = rng.byte(128, 255);
        let mut s1 = rng.bytes(prefix_length, 1, 127);
        s1.push(hit);
        s1.extend(rng.bytes(suffix_length, 1, 255));
        let s2_length = rng.usize(1, 128);
        let mut s2 = rng.bytes(s2_length, 128, 255);
        let hit_index = rng.usize(0, s2.len() - 1);
        s2[hit_index] = hit;
        compare_case(4, case_number, c_driver, rust_driver, s1, s2);
    }

    for case_number in 0..256 {
        let s1_length = rng.usize(1, 1024);
        let s2_length = rng.usize(1, 128);
        let s1 = rng.bytes(s1_length, 1, 127);
        let s2 = rng.bytes(s2_length, 128, 255);
        compare_case(5, case_number, c_driver, rust_driver, s1, s2);
    }

    for case_number in 0..256 {
        let first = rng.byte(1, 127);
        let hit = rng.byte(128, 255);
        let repeats = rng.usize(2, 512);
        let mut s1 = vec![first; repeats];
        s1.extend([hit, hit, hit]);
        let mut s2 = vec![hit; rng.usize(2, 128)];
        s2.extend([first.wrapping_add(1), hit]);
        compare_case(6, case_number, c_driver, rust_driver, s1, s2);
    }

    compare_case(
        5,
        256,
        c_driver,
        rust_driver,
        vec![1; 65_536],
        vec![255; 4_096],
    );
}

fn run_null_child(library: &Path, null_argument: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("DRIVER_NULL_CHILD_LIBRARY", library)
        .env("DRIVER_NULL_ARGUMENT", null_argument)
        .status()
        .expect("run isolated null-pointer child")
}

#[test]
fn null_pointer_behavior_matches_in_isolated_processes() {
    for null_argument in ["s1", "s2"] {
        let c_status = run_null_child(&c_library_path(), null_argument);
        let rust_status = run_null_child(&rust_library_path(), null_argument);
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {null_argument}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly accepted null {null_argument}"
        );
        let c_signal = c_status
            .signal()
            .expect("C null-pointer call must terminate by signal");
        let rust_signal = rust_status
            .signal()
            .expect("Rust null-pointer call must terminate by signal");
        assert_eq!(
            rust_signal, c_signal,
            "null {null_argument} terminated differently"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_path) = std::env::var("DRIVER_NULL_CHILD_LIBRARY") else {
        return;
    };
    let null_argument =
        std::env::var("DRIVER_NULL_ARGUMENT").expect("null child argument selector");
    let (_library, driver) = unsafe { load_driver(Path::new(&library_path)) };
    let nonnull = CString::new("nonnull").unwrap();

    unsafe {
        match null_argument.as_str() {
            "s1" => driver(std::ptr::null(), nonnull.as_ptr()),
            "s2" => driver(nonnull.as_ptr(), std::ptr::null()),
            _ => panic!("unknown null argument selector"),
        }
    }
}
