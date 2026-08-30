use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir.join("../c_src/build/libdriver.so");
        let rust_path = manifest_dir.join("target/release/libdriver.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn drivers(&self) -> (Symbol<'_, Driver>, Symbol<'_, Driver>) {
        (
            unsafe { self.c.get(b"driver\0").expect("load C driver symbol") },
            unsafe { self.rust.get(b"driver\0").expect("load Rust driver symbol") },
        )
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library does not exist: {}; build both release libraries first",
        path.display()
    );
}

fn stdout_lock() -> MutexGuard<'static, ()> {
    STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "flush stdout");

    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "create pipe");

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(pipe_fds[1], STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close pipe writer");

    call();

    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "flush captured output"
    );
    assert_eq!(
        unsafe { dup2(saved_stdout, STDOUT_FILENO) },
        STDOUT_FILENO,
        "restore stdout"
    );
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

unsafe fn compare_driver(c_driver: Driver, rust_driver: Driver, x: c_int) {
    let c_output = unsafe { capture_stdout(|| c_driver(x)) };
    let rust_output = unsafe { capture_stdout(|| rust_driver(x)) };
    assert_eq!(rust_output, c_output, "output differs for x={x}");
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn config_1_negative_x_has_identical_empty_output() {
    let _stdout = stdout_lock();
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.drivers() };

    for x in [c_int::MIN, -65_536, -2, -1] {
        unsafe { compare_driver(*c_driver, *rust_driver, x) };
    }

    let mut state = 0x4e45_4741_5449_5645;
    for _ in 0..256 {
        let x = -1 - (next_random(&mut state) % 65_536) as c_int;
        unsafe { compare_driver(*c_driver, *rust_driver, x) };
    }
}

#[test]
fn config_2_zero_x_has_identical_empty_output() {
    let _stdout = stdout_lock();
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.drivers() };

    unsafe { compare_driver(*c_driver, *rust_driver, 0) };
}

#[test]
fn config_3_one_x_has_identical_single_line_output() {
    let _stdout = stdout_lock();
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.drivers() };

    unsafe { compare_driver(*c_driver, *rust_driver, 1) };
}

#[test]
fn config_4_many_x_values_have_identical_output() {
    let _stdout = stdout_lock();
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.drivers() };

    for x in [2, 3, 127, 128, 255, 256] {
        unsafe { compare_driver(*c_driver, *rust_driver, x) };
    }

    let mut state = 0x4d41_4e59_5f58_5345;
    for _ in 0..256 {
        let x = 2 + (next_random(&mut state) % 255) as c_int;
        unsafe { compare_driver(*c_driver, *rust_driver, x) };
    }
}
