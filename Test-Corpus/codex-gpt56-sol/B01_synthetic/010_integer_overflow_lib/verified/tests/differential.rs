use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
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

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = env::var_os("DRIVER_RUST_SO") {
        return path.into();
    }

    let test_executable = env::current_exe().expect("locate integration test executable");
    let deps_dir = test_executable
        .parent()
        .expect("integration test executable has a parent");
    let candidates = [
        deps_dir.join("libdriver.so"),
        deps_dir
            .parent()
            .expect("target profile directory exists")
            .join("libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .expect("locate the Rust libdriver.so; run cargo build before cargo test")
}

fn shuffled_char_domain() -> Vec<c_char> {
    let mut values: Vec<c_char> = (0_u16..=u8::MAX as u16)
        .map(|value| value as u8 as c_char)
        .collect();
    let mut state = 0x6a09_e667_f3bc_c909_u64;

    for index in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.swap(index, state as usize % (index + 1));
    }

    values
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
    }

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "redirect stdout");
    assert_eq!(
        unsafe { close(pipe_fds[1]) },
        0,
        "close duplicate pipe writer"
    );

    call();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn compare_symbol(symbol_name: &[u8]) {
    let _stdout_guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let c_path = c_library_path();
    let rust_path = rust_library_path();

    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );
    assert_ne!(
        c_path.canonicalize().expect("canonicalize C library"),
        rust_path.canonicalize().expect("canonicalize Rust library"),
        "C and Rust paths must identify different shared libraries"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_function: Symbol<CharFunction> =
            c_library.get(symbol_name).expect("resolve C symbol");
        let rust_function: Symbol<CharFunction> =
            rust_library.get(symbol_name).expect("resolve Rust symbol");

        for value in shuffled_char_domain() {
            let c_output = capture_stdout(|| c_function(value));
            let rust_output = capture_stdout(|| rust_function(value));
            assert_eq!(
                rust_output,
                c_output,
                "output mismatch for symbol {:?} and char byte 0x{:02x}",
                String::from_utf8_lossy(&symbol_name[..symbol_name.len() - 1]),
                value as u8
            );
        }
    }
}

#[test]
fn print_hex_char_line_matches_for_all_char_values() {
    compare_symbol(b"printHexCharLine\0");
}

#[test]
fn driver_matches_for_all_char_values() {
    compare_symbol(b"driver\0");
}
