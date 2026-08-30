use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: RawFd = 1;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn shared_objects() -> (PathBuf, PathBuf) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libdriver.so"),
        crate_dir.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [0; 2];
    unsafe {
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-call fflush failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "stdout redirect failed"
        );
        assert_eq!(close(pipe_fds[1]), 0, "write descriptor close failed");

        action();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "post-call fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "stdout restore failed"
        );
        assert_eq!(close(saved_stdout), 0, "saved descriptor close failed");
    }

    let mut output = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let bytes_read = unsafe {
            read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            )
        };
        assert!(bytes_read >= 0, "read failed");
        if bytes_read == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..bytes_read as usize]);
    }
    unsafe {
        assert_eq!(close(pipe_fds[0]), 0, "read descriptor close failed");
    }
    output
}

fn deterministic_inputs() -> Vec<c_int> {
    let mut inputs = vec![
        0,
        1,
        -1,
        c_int::MIN,
        c_int::MAX,
        0x0102_0304,
        0x7f00_ff80,
        0x5555_5555,
        0xaaaa_aaaa_u32 as c_int,
    ];

    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32 as c_int);
    }
    inputs
}

#[test]
fn config_1_all_int_bit_patterns_match_byte_for_byte() {
    let (c_path, rust_path) = shared_objects();
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

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared object");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared object");
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0") }.expect("load C driver");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver");
    let inputs = deterministic_inputs();

    let c_output = capture_stdout(|| {
        for &input in &inputs {
            unsafe { c_driver(input) };
        }
    });
    let rust_output = capture_stdout(|| {
        for &input in &inputs {
            unsafe { rust_driver(input) };
        }
    });

    assert_eq!(rust_output, c_output);
    assert_eq!(c_output.len(), inputs.len() * (size_of::<c_int>() * 2 + 1));
}
