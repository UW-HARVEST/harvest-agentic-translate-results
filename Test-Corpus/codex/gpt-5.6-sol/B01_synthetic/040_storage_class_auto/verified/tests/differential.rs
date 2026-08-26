use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);
type Main = unsafe extern "C" fn() -> c_int;

static STDIO_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn __fpurge(stream: *mut c_void);
    static mut stdin: *mut c_void;
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let c_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "missing C shared library {}; build it before testing",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn drivers(&self) -> (Symbol<'_, Driver>, Symbol<'_, Driver>) {
        (
            unsafe { self.c.get(b"driver\0").expect("C driver export") },
            unsafe { self.rust.get(b"driver\0").expect("Rust driver export") },
        )
    }

    unsafe fn mains(&self) -> (Symbol<'_, Main>, Symbol<'_, Main>) {
        (
            unsafe { self.c.get(b"main\0").expect("C main export") },
            unsafe { self.rust.get(b"main\0").expect("Rust main export") },
        )
    }
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    test_executable
        .parent()
        .expect("test executable directory")
        .join("libdriver.so")
}

fn make_pipe() -> [c_int; 2] {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    fds
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "initial fflush failed"
    );

    let [read_fd, write_fd] = make_pipe();
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout failed");
    assert_eq!(unsafe { dup2(write_fd, 1) }, 1, "redirect stdout failed");
    assert_eq!(unsafe { close(write_fd) }, 0, "close pipe writer failed");

    let result = operation();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "final fflush failed");
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout failed");
    assert_eq!(
        unsafe { close(saved_stdout) },
        0,
        "close saved stdout failed"
    );

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(read_fd) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    (result, output)
}

fn with_stdin<T>(input: &[u8], operation: impl FnOnce() -> T) -> T {
    let [read_fd, write_fd] = make_pipe();
    let mut writer = unsafe { File::from_raw_fd(write_fd) };
    writer.write_all(input).expect("write redirected stdin");
    drop(writer);

    let saved_stdin = unsafe { dup(0) };
    assert!(saved_stdin >= 0, "dup stdin failed");

    let stream = unsafe { stdin };
    unsafe {
        __fpurge(stream);
        clearerr(stream);
    }
    assert_eq!(unsafe { dup2(read_fd, 0) }, 0, "redirect stdin failed");
    assert_eq!(unsafe { close(read_fd) }, 0, "close pipe reader failed");

    let result = operation();

    unsafe {
        __fpurge(stream);
        clearerr(stream);
    }
    assert_eq!(unsafe { dup2(saved_stdin, 0) }, 0, "restore stdin failed");
    assert_eq!(unsafe { close(saved_stdin) }, 0, "close saved stdin failed");
    unsafe {
        clearerr(stream);
    }
    result
}

fn run_driver(driver: Driver, values: &[c_int]) -> Vec<u8> {
    capture_stdout(|| {
        for &value in values {
            unsafe { driver(value) };
        }
    })
    .1
}

fn run_main_with_input(main: Main, input: &[u8]) -> (c_int, Vec<u8>) {
    let (result, output) = capture_stdout(|| with_stdin(input, || unsafe { main() }));
    (result, output)
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state as u32
}

#[test]
fn config_1_driver_matches_for_randomized_full_width_ints() {
    let _stdio = STDIO_LOCK.lock().expect("stdio lock");
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.drivers() };

    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1_073_741_975,
        -1_073_741_974,
        -1,
        0,
        1,
        1_073_741_673,
        1_073_741_674,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..1_024 {
        values.push(next_random(&mut state) as c_int);
    }

    let c_output = run_driver(*c_driver, &values);
    let rust_output = run_driver(*rust_driver, &values);
    assert_eq!(rust_output, c_output);
}

#[test]
fn config_2_main_matches_for_randomized_valid_decimal_input() {
    let _stdio = STDIO_LOCK.lock().expect("stdio lock");
    let libraries = unsafe { Libraries::load() };
    let (c_main, rust_main) = unsafe { libraries.mains() };

    let mut state = 0x8a5c_d789_635d_2dff_u64;
    for index in 0..128 {
        let value = (next_random(&mut state) % 2_000_001) as c_int - 1_000_000;
        let input = match index % 4 {
            0 => format!("{value}\n"),
            1 => format!(" \t{value}\n"),
            2 if value >= 0 => format!("+{value} trailing"),
            2 => format!("{value} trailing"),
            _ => format!("{value}\nignored"),
        };

        let c_result = run_main_with_input(*c_main, input.as_bytes());
        let rust_result = run_main_with_input(*rust_main, input.as_bytes());
        assert_eq!(rust_result, c_result, "input {input:?}");
    }
}

#[test]
fn config_3_main_matches_for_randomized_nonnumeric_input() {
    let _stdio = STDIO_LOCK.lock().expect("stdio lock");
    let libraries = unsafe { Libraries::load() };
    let (c_main, rust_main) = unsafe { libraries.mains() };

    let mut state = 0xd1b5_4a32_d192_ed03_u64;
    for _ in 0..128 {
        let length = (next_random(&mut state) % 31 + 1) as usize;
        let mut input = Vec::with_capacity(length + 1);
        for _ in 0..length {
            input.push(b'a' + (next_random(&mut state) % 26) as u8);
        }
        input.push(b'\n');

        let c_result = run_main_with_input(*c_main, &input);
        let rust_result = run_main_with_input(*rust_main, &input);
        assert_eq!(rust_result, c_result, "input {input:?}");
    }
}

#[test]
fn config_4_main_matches_for_eof() {
    let _stdio = STDIO_LOCK.lock().expect("stdio lock");
    let libraries = unsafe { Libraries::load() };
    let (c_main, rust_main) = unsafe { libraries.mains() };

    for _ in 0..64 {
        let c_result = run_main_with_input(*c_main, b"");
        let rust_result = run_main_with_input(*rust_main, b"");
        assert_eq!(rust_result, c_result);
    }
}
