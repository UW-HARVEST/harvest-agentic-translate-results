use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn assert_syscall_succeeded(result: c_int, operation: &str) {
    assert!(
        result >= 0,
        "{operation} failed: {}",
        std::io::Error::last_os_error()
    );
}

fn capture_native_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture mutex poisoned");
    let mut pipe_fds = [-1; 2];

    assert_eq!(
        unsafe { fflush(std::ptr::null_mut()) },
        0,
        "fflush before capture failed"
    );
    assert_syscall_succeeded(unsafe { pipe(pipe_fds.as_mut_ptr()) }, "pipe");

    let saved_stdout = unsafe { dup(1) };
    assert_syscall_succeeded(saved_stdout, "dup");
    assert_syscall_succeeded(unsafe { dup2(pipe_fds[1], 1) }, "dup2 redirect");
    assert_syscall_succeeded(unsafe { close(pipe_fds[1]) }, "close pipe writer");

    let read_fd = pipe_fds[0];
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let bytes_read =
                unsafe { read(read_fd, buffer.as_mut_ptr().cast::<c_void>(), buffer.len()) };
            assert!(
                bytes_read >= 0,
                "read failed: {}",
                std::io::Error::last_os_error()
            );
            if bytes_read == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..bytes_read as usize]);
        }
        assert_syscall_succeeded(unsafe { close(read_fd) }, "close pipe reader");
        output
    });

    call();

    assert_eq!(
        unsafe { fflush(std::ptr::null_mut()) },
        0,
        "fflush after call failed"
    );
    assert_syscall_succeeded(unsafe { dup2(saved_stdout, 1) }, "dup2 restore");
    assert_syscall_succeeded(unsafe { close(saved_stdout) }, "close saved stdout");
    reader.join().expect("stdout reader thread panicked")
}

fn fixed_seed_inputs() -> Vec<c_int> {
    let mut inputs = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65_535,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32 as c_int);
    }
    inputs
}

unsafe fn run_library(path: &Path, inputs: &[c_int]) -> Vec<u8> {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let driver: Symbol<'_, Driver> = unsafe { library.get(b"driver\0") }
        .unwrap_or_else(|error| panic!("failed to load driver from {}: {error}", path.display()));

    capture_native_stdout(|| {
        for &input in inputs {
            unsafe { driver(input) };
        }
    })
}

#[test]
fn driver_matches_c_for_full_configuration_surface() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust release shared library: {}",
        rust_path.display()
    );

    let inputs = fixed_seed_inputs();
    let c_output = unsafe { run_library(&c_path, &inputs) };
    let rust_output = unsafe { run_library(&rust_path, &inputs) };

    const BYTES_PER_CALL: usize = 33;
    assert_eq!(c_output.len(), inputs.len() * BYTES_PER_CALL);
    assert_eq!(rust_output.len(), inputs.len() * BYTES_PER_CALL);

    if c_output != rust_output {
        let differing_call = c_output
            .chunks_exact(BYTES_PER_CALL)
            .zip(rust_output.chunks_exact(BYTES_PER_CALL))
            .position(|(c, rust)| c != rust)
            .expect("outputs differ but no differing call was found");
        panic!(
            "output mismatch for input {}: C={:?}, Rust={:?}",
            inputs[differing_call],
            &c_output[differing_call * BYTES_PER_CALL..(differing_call + 1) * BYTES_PER_CALL],
            &rust_output[differing_call * BYTES_PER_CALL..(differing_call + 1) * BYTES_PER_CALL],
        );
    }
}
