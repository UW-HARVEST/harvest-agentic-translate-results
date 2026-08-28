use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

const STDOUT_FILENO: c_int = 1;
const TRIALS: usize = 64;

type HelloWorld = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn library_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(relative)
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to locate {relative}: {error}"))
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "failed to flush stdout");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "failed to create pipe");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "failed to duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "failed to redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "failed to close pipe writer");

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0, "failed to flush output");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "failed to restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "failed to close saved stdout");

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader
            .read_to_end(&mut output)
            .expect("failed to read captured stdout");
        (result, output)
    }
}

fn call_batch(library: &Library, count: usize) -> (Vec<c_int>, Vec<u8>) {
    let helloworld: Symbol<HelloWorld> =
        unsafe { library.get(b"helloworld\0") }.expect("missing helloworld export");

    capture_stdout(|| {
        (0..count)
            .map(|_| unsafe { helloworld() })
            .collect::<Vec<_>>()
    })
}

#[test]
fn helloworld_matches_c_across_repeated_calls() {
    let c_path = library_path("../c_src/build/libhello.so");
    let rust_path = library_path("target/release/libhello.so");
    assert_ne!(c_path, rust_path, "C and Rust library paths must differ");

    let c_library = unsafe { Library::new(&c_path) }.expect("failed to load C shared library");
    let rust_library =
        unsafe { Library::new(&rust_path) }.expect("failed to load Rust shared library");

    let mut random_state = 0x4d59_5df4_d0f3_3173_u64;
    for trial in 0..TRIALS {
        random_state ^= random_state << 13;
        random_state ^= random_state >> 7;
        random_state ^= random_state << 17;
        let call_count = (random_state as usize % 32) + 1;

        let (c_results, c_output) = call_batch(&c_library, call_count);
        let (rust_results, rust_output) = call_batch(&rust_library, call_count);

        assert_eq!(rust_results, c_results, "return mismatch in trial {trial}");
        assert_eq!(rust_output, c_output, "output mismatch in trial {trial}");
    }
}
