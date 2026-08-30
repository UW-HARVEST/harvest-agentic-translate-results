use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type CharFunction = unsafe extern "C" fn(c_char);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

struct Api {
    _library: Library,
    print_hex_char_line: CharFunction,
    driver: CharFunction,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_hex_char_line = unsafe {
            *library
                .get::<CharFunction>(b"printHexCharLine\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load printHexCharLine from {}: {error}",
                        path.display()
                    )
                })
        };
        let driver = unsafe {
            *library
                .get::<CharFunction>(b"driver\0")
                .unwrap_or_else(|error| {
                    panic!("failed to load driver from {}: {error}", path.display())
                })
        };

        Self {
            _library: library,
            print_hex_char_line,
            driver,
        }
    }
}

fn rust_library_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("target"));
    target_dir.join("release").join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn shuffled_bytes() -> [u8; 256] {
    let mut values = [0_u8; 256];
    for (index, value) in values.iter_mut().enumerate() {
        *value = index as u8;
    }

    let mut state = 0x8f4d_3b2a_1907_e6c5_u64;
    for index in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.swap(index, (state as usize) % (index + 1));
    }
    values
}

fn capture_stdout(function: CharFunction, value: c_char) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    let _lock = STDOUT_LOCK.lock().expect("stdout lock was poisoned");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before capture");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup stdout");
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO, "dup2 pipe");
        assert_eq!(close(pipe_fds[1]), 0, "close pipe writer");

        function(value);

        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush after call");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        output
    }
}

#[test]
fn every_configuration_matches_for_all_char_bit_patterns() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_api = unsafe { Api::load(&c_path) };
    let rust_api = unsafe { Api::load(&rust_path) };

    for value in shuffled_bytes() {
        let value = value as c_char;
        assert_eq!(
            capture_stdout(c_api.print_hex_char_line, value),
            capture_stdout(rust_api.print_hex_char_line, value),
            "printHexCharLine diverged for char bit pattern 0x{:02x}",
            value as u8
        );
    }

    for value in shuffled_bytes() {
        let value = value as c_char;
        assert_eq!(
            capture_stdout(c_api.driver, value),
            capture_stdout(rust_api.driver, value),
            "driver diverged for char bit pattern 0x{:02x}",
            value as u8
        );
    }
}
