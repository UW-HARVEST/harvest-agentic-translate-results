use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

const STDIN_FILENO: RawFd = 0;
const STDOUT_FILENO: RawFd = 1;
const RANDOM_CASES: usize = 64;

static FFI_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;

    static mut stdin: *mut c_void;
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }
}

struct FdRestore {
    saved_fd: RawFd,
    target_fd: RawFd,
}

impl FdRestore {
    fn redirect(target_fd: RawFd, replacement_fd: RawFd) -> Self {
        let saved_fd = unsafe { dup(target_fd) };
        assert!(saved_fd >= 0, "dup({target_fd}) failed");
        assert_eq!(
            unsafe { dup2(replacement_fd, target_fd) },
            target_fd,
            "dup2 to fd {target_fd} failed"
        );
        Self {
            saved_fd,
            target_fd,
        }
    }
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { dup2(self.saved_fd, self.target_fd) },
            self.target_fd,
            "failed to restore fd {}",
            self.target_fd
        );
        assert_eq!(unsafe { close(self.saved_fd) }, 0, "close failed");
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory")
        .join("libdriver.so")
}

fn temp_file(label: &str) -> (PathBuf, File) {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}-{label}",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("open temporary file");
    (path, file)
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let (path, mut output) = temp_file("stdout");
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);

    let result = {
        let _stdout = FdRestore::redirect(STDOUT_FILENO, output.as_raw_fd());
        let result = call();
        assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
        result
    };

    output.seek(SeekFrom::Start(0)).expect("rewind stdout");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read stdout");
    drop(output);
    fs::remove_file(path).expect("remove stdout file");
    (result, bytes)
}

fn run_driver(library_path: &Path, x: i32) -> Vec<u8> {
    let library = unsafe { Library::new(library_path) }.expect("load shared library");
    let driver: Symbol<'_, DriverFn> = unsafe { library.get(b"driver\0") }.expect("load driver");
    let (_, output) = capture_stdout(|| unsafe { driver(x) });
    output
}

fn run_main(library_path: &Path, input: &[u8]) -> (i32, Vec<u8>) {
    let (path, mut input_file) = temp_file("stdin");
    input_file.write_all(input).expect("write stdin");
    input_file.seek(SeekFrom::Start(0)).expect("rewind stdin");

    let result = {
        let _stdin = FdRestore::redirect(STDIN_FILENO, input_file.as_raw_fd());
        unsafe {
            clearerr(stdin);
        }

        let library = unsafe { Library::new(library_path) }.expect("load shared library");
        let main: Symbol<'_, MainFn> = unsafe { library.get(b"main\0") }.expect("load main");
        capture_stdout(|| unsafe { main() })
    };

    unsafe {
        clearerr(stdin);
    }
    drop(input_file);
    fs::remove_file(path).expect("remove stdin file");
    result
}

fn assert_driver_matches(x: i32) {
    let c_output = run_driver(&c_library_path(), x);
    let rust_output = run_driver(&rust_library_path(), x);
    assert_eq!(rust_output, c_output, "driver diverged for x={x}");
}

fn assert_main_matches(input: &[u8]) -> (i32, Vec<u8>) {
    let c_result = run_main(&c_library_path(), input);
    let rust_result = run_main(&rust_library_path(), input);
    assert_eq!(rust_result, c_result, "main diverged for input {input:?}");
    rust_result
}

#[test]
fn config_1_driver_negative() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let mut rng = Lcg::new(0x78ad_60ce_2f49_128b);
    for _ in 0..RANDOM_CASES {
        let x = -1 - (rng.next_u32() % 100_000) as i32;
        assert_driver_matches(x);
    }
}

#[test]
fn config_2_driver_zero() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    for _ in 0..RANDOM_CASES {
        assert_driver_matches(0);
    }
}

#[test]
fn config_3_driver_one() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    for _ in 0..RANDOM_CASES {
        assert_driver_matches(1);
    }
}

#[test]
fn config_4_driver_many() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let mut rng = Lcg::new(0xf1bd_81b4_64f1_fa69);
    for _ in 0..RANDOM_CASES {
        let x = 2 + (rng.next_u32() % 127) as i32;
        assert_driver_matches(x);
    }
}

#[test]
fn config_5_main_negative() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let mut rng = Lcg::new(0x55cd_23bd_92a4_3c81);
    for case in 0..RANDOM_CASES {
        let value = -1 - (rng.next_u32() % 100_000) as i32;
        let input = match case % 3 {
            0 => format!("{value}"),
            1 => format!(" \t{value}\n"),
            _ => format!("\n{value} trailing"),
        };
        assert_eq!(assert_main_matches(input.as_bytes()), (0, Vec::new()));
    }
}

#[test]
fn config_6_main_zero() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let inputs: [&[u8]; 6] = [b"0", b"+0", b"-0", b"000", b" \t0\n", b"\n+000 tail"];
    for case in 0..RANDOM_CASES {
        assert_eq!(
            assert_main_matches(inputs[case % inputs.len()]),
            (0, Vec::new())
        );
    }
}

#[test]
fn config_7_main_one() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let inputs: [&[u8]; 6] = [b"1", b"+1", b"01", b" \t1\n", b"\n+001 tail", b"1 999"];
    for case in 0..RANDOM_CASES {
        let (result, output) = assert_main_matches(inputs[case % inputs.len()]);
        assert_eq!(result, 0);
        assert_eq!(output, b"0 0\n");
    }
}

#[test]
fn config_8_main_many() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let mut rng = Lcg::new(0x8c0e_b6f7_a811_f250);
    for case in 0..RANDOM_CASES {
        let value = 2 + (rng.next_u32() % 63) as i32;
        let input = match case % 4 {
            0 => format!("{value}"),
            1 => format!("+{value}"),
            2 => format!(" \n00{value} trailing"),
            _ => format!("{value}\n999"),
        };
        let (result, output) = assert_main_matches(input.as_bytes());
        assert_eq!(result, 0);
        assert_eq!(
            output.iter().filter(|&&byte| byte == b'\n').count(),
            value as usize
        );
    }
}

#[test]
fn error_1_main_eof_before_conversion() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let mut rng = Lcg::new(0x657a_d002_5707_438a);
    for case in 0..RANDOM_CASES {
        let whitespace = [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c];
        let mut input = Vec::new();
        if case != 0 {
            for _ in 0..(rng.next_u32() % 32) {
                input.push(whitespace[(rng.next_u32() as usize) % whitespace.len()]);
            }
        }
        assert_eq!(assert_main_matches(&input), (0, Vec::new()));
    }
}

#[test]
fn error_2_main_matching_failure() {
    let _guard = FFI_LOCK.lock().expect("FFI lock");
    let prefixes: [&[u8]; 8] = [
        b"x",
        b"abc",
        b".1",
        b"--1",
        b"+x",
        b"-x",
        b" \t!",
        b"\nword 12",
    ];
    let mut rng = Lcg::new(0xe60f_01ee_8fc0_c218);
    for case in 0..RANDOM_CASES {
        let mut input = prefixes[case % prefixes.len()].to_vec();
        input.extend_from_slice(&(rng.next_u32() % 100_000).to_string().into_bytes());
        assert_eq!(assert_main_matches(&input), (0, Vec::new()));
    }
}
