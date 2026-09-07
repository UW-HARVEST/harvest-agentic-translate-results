use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

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

fn shared_object_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-capture fflush failed");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            dup2(write_fd, STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );
        assert_eq!(close(write_fd), 0, "closing duplicate pipe writer failed");

        call();
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "captured-output fflush failed"
        );

        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(read_fd);
        reader
            .read_to_end(&mut output)
            .expect("reading captured stdout failed");
        output
    }
}

fn input_corpus() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -0x0102_0304,
        -256,
        -255,
        -2,
        -1,
        0,
        1,
        2,
        15,
        16,
        127,
        128,
        255,
        256,
        0x0102_0304,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    // Fixed-seed xorshift64* corpus supplies many reproducible full-width
    // object representations without adding a second test dependency.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..4096 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        values.push(random as u32 as c_int);
    }
    values
}

#[test]
fn config_1_driver_matches_for_all_sampled_int_object_representations() {
    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let (c_path, rust_path) = shared_object_paths();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("loading C shared object failed");
    let rust_library =
        unsafe { Library::new(&rust_path) }.expect("loading Rust shared object failed");
    let c_driver: Driver = unsafe {
        *c_library
            .get::<Driver>(b"driver\0")
            .expect("loading C driver symbol failed")
    };
    let rust_driver: Driver = unsafe {
        *rust_library
            .get::<Driver>(b"driver\0")
            .expect("loading Rust driver symbol failed")
    };

    for input in input_corpus() {
        let c_output = capture_stdout(|| unsafe { c_driver(input) });
        let rust_output = capture_stdout(|| unsafe { rust_driver(input) });
        assert_eq!(
            rust_output, c_output,
            "stdout mismatch for input {input} (bits {:#010x})",
            input as u32
        );
    }
}
