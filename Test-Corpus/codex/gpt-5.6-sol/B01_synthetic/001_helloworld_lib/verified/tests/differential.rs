use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io;
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};

type HelloWorld = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: RawFd = 1;

struct StdoutRestore(RawFd);

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.0, STDOUT_FILENO);
            close(self.0);
        }
    }
}

fn capture_stdout(call: HelloWorld, repetitions: usize) -> io::Result<(Vec<u8>, Vec<c_int>)> {
    let mut pipe_fds = [0; 2];
    if unsafe { pipe(pipe_fds.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    if saved_stdout == -1 {
        unsafe {
            close(pipe_fds[0]);
            close(pipe_fds[1]);
        }
        return Err(io::Error::last_os_error());
    }
    let restore = StdoutRestore(saved_stdout);

    if unsafe { fflush(std::ptr::null_mut()) } != 0
        || unsafe { dup2(pipe_fds[1], STDOUT_FILENO) } == -1
    {
        unsafe {
            close(pipe_fds[0]);
            close(pipe_fds[1]);
        }
        return Err(io::Error::last_os_error());
    }
    unsafe {
        close(pipe_fds[1]);
    }

    let returns = (0..repetitions).map(|_| unsafe { call() }).collect();
    if unsafe { fflush(std::ptr::null_mut()) } != 0 {
        unsafe {
            close(pipe_fds[0]);
        }
        return Err(io::Error::last_os_error());
    }

    drop(restore);

    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let bytes_read = unsafe { read(pipe_fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
        if bytes_read == 0 {
            break;
        }
        if bytes_read == -1 {
            unsafe {
                close(pipe_fds[0]);
            }
            return Err(io::Error::last_os_error());
        }
        output.extend_from_slice(&buffer[..bytes_read as usize]);
    }
    unsafe {
        close(pipe_fds[0]);
    }

    Ok((output, returns))
}

fn rust_library_path(crate_root: &Path) -> PathBuf {
    crate_root.join("target/release/libhello.so")
}

#[test]
fn helloworld_matches_c_through_shared_library_exports() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = crate_root.join("c_src/build/libhello.so");
    let rust_path = rust_library_path(&crate_root);

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

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_helloworld: Symbol<HelloWorld> =
        unsafe { c_library.get(b"helloworld\0") }.expect("resolve C helloworld");
    let rust_helloworld: Symbol<HelloWorld> =
        unsafe { rust_library.get(b"helloworld\0") }.expect("resolve Rust helloworld");

    // The API has no input values to randomize. A fixed-seed generator varies
    // repetition counts to verify repeat calls and accumulated output.
    let mut seed = 0x5eed_cafe_d15c_a11u64;
    for _ in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let repetitions = (seed as usize % 32) + 1;

        let c_result = capture_stdout(*c_helloworld, repetitions).expect("capture C output");
        let rust_result =
            capture_stdout(*rust_helloworld, repetitions).expect("capture Rust output");

        assert_eq!(rust_result.1, c_result.1, "return values differ");
        assert_eq!(rust_result.0, c_result.0, "stdout bytes differ");
        assert_eq!(c_result.1, vec![0; repetitions]);
        assert_eq!(c_result.0, b"Hello World!\n".repeat(repetitions));
    }
}
