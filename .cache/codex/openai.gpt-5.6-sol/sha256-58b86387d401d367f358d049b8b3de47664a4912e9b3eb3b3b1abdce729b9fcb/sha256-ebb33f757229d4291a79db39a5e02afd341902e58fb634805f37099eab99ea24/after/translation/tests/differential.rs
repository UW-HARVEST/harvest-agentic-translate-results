use libloading::{Library, Symbol};
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
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
        })
}

fn randomized_full_domain(seed: u64, random_count: usize) -> Vec<c_char> {
    let mut state = seed;
    let mut values: Vec<c_char> = (0_u16..=255).map(|value| value as u8 as c_char).collect();

    for index in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.swap(index, state as usize % (index + 1));
    }

    for _ in 0..random_count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as u8 as c_char);
    }

    values
}

fn capture_stdout(operation: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe");

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        operation();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush captured output");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        output
    }
}

fn call_from_library(path: &Path, symbol_name: &[u8], inputs: &[c_char]) -> Vec<u8> {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );

    unsafe {
        let library =
            Library::new(path).unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
        let function: Symbol<'_, CharFunction> = library
            .get(symbol_name)
            .unwrap_or_else(|error| panic!("load symbol {:?}: {error}", symbol_name));

        capture_stdout(|| {
            for &input in inputs {
                function(input);
            }
        })
    }
}

fn assert_symbol_matches(symbol_name: &[u8], seed: u64) {
    let inputs = randomized_full_domain(seed, 4096);
    let c_output = call_from_library(&c_library_path(), symbol_name, &inputs);
    let rust_output = call_from_library(&rust_library_path(), symbol_name, &inputs);

    assert_eq!(
        rust_output,
        c_output,
        "byte output differs for symbol {:?} across {} inputs",
        symbol_name,
        inputs.len()
    );
}

#[test]
fn configuration_surface_matches_c_byte_for_byte() {
    assert_symbol_matches(b"printHexCharLine\0", 0x4d59_5df4_d0f3_3173);
    assert_symbol_matches(b"driver\0", 0x8f4d_3b2a_1907_e6c5);
}
