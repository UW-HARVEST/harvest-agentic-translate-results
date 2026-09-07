use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: RawFd = 1;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

fn stdout_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct StdoutRestore {
    saved_fd: RawFd,
}

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            let _ = fflush(std::ptr::null_mut());
            let _ = dup2(self.saved_fd, STDOUT_FILENO);
            let _ = close(self.saved_fd);
        }
    }
}

fn capture_stdout(mut invoke: impl FnMut()) -> Vec<u8> {
    let _lock = stdout_lock().lock().expect("stdout capture lock poisoned");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-capture fflush failed");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_fd = dup(STDOUT_FILENO);
        assert!(saved_fd >= 0, "dup stdout failed");
        let restore = StdoutRestore { saved_fd };

        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close duplicated pipe writer failed");

        invoke();
        assert_eq!(fflush(std::ptr::null_mut()), 0, "captured fflush failed");

        assert_eq!(
            dup2(restore.saved_fd, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout failed"
        );
        assert_eq!(close(restore.saved_fd), 0, "close saved stdout failed");
        std::mem::forget(restore);

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(count >= 0, "read captured stdout failed");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader failed");
        output
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("resolve test executable");
    test_executable
        .parent()
        .and_then(|deps| deps.parent())
        .expect("resolve Cargo profile directory")
        .join("libdriver.so")
}

fn generated_inputs() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    // Fixed-seed SplitMix64 gives reproducible coverage over the complete
    // 32-bit C int bit pattern, including many overflow-producing operands.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..16_384 {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^= mixed >> 31;
        values.push(mixed as u32 as c_int);
    }
    values
}

#[test]
fn driver_matches_for_integer_boundaries_and_randomized_inputs() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    let c_library = unsafe { Library::new(&c_path).expect("load C shared library") };
    let rust_library = unsafe { Library::new(&rust_path).expect("load Rust shared library") };
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0").expect("load C driver symbol") };
    let rust_driver: Symbol<'_, Driver> = unsafe {
        rust_library
            .get(b"driver\0")
            .expect("load Rust driver symbol")
    };

    let inputs = generated_inputs();
    for (chunk_index, chunk) in inputs.chunks(512).enumerate() {
        let c_output = capture_stdout(|| {
            for &value in chunk {
                unsafe { c_driver(value) };
            }
        });
        let rust_output = capture_stdout(|| {
            for &value in chunk {
                unsafe { rust_driver(value) };
            }
        });

        assert_eq!(
            c_output.iter().filter(|&&byte| byte == b'\n').count(),
            chunk.len(),
            "C stdout capture did not contain one line per call in chunk {chunk_index}"
        );
        assert_eq!(
            rust_output, c_output,
            "stdout mismatch in randomized input chunk {chunk_index}"
        );
    }
}
