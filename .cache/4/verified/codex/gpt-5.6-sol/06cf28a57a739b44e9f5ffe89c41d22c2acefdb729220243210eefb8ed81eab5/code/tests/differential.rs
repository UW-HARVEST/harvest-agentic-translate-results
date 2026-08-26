use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_CAPTURE_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libdriver.so"),
        root.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_CAPTURE_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .unwrap();
    }
    output
}

fn test_values() -> Vec<c_int> {
    let mut values = vec![
        0,
        1,
        -1,
        c_int::MIN,
        c_int::MAX,
        0x0123_4567,
        0x7654_3210,
        0x00ff_00ff,
        0x7f80_ff01,
        0x8000_0000_u32 as c_int,
        0xffff_0000_u32 as c_int,
        0xa55a_c33c_u32 as c_int,
    ];

    let mut state = 0x6d2b_79f5_u32;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        values.push(state as c_int);
    }
    values
}

#[test]
fn configuration_1_driver_all_int_values_match_byte_for_byte() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    let c_library = unsafe { Library::new(c_path).unwrap() };
    let rust_library = unsafe { Library::new(rust_path).unwrap() };
    let c_driver = unsafe { c_library.get::<Driver>(b"driver\0").unwrap() };
    let rust_driver = unsafe { rust_library.get::<Driver>(b"driver\0").unwrap() };
    let values = test_values();

    let c_output = capture_stdout(|| {
        for &value in &values {
            unsafe { c_driver(value) };
        }
    });
    let rust_output = capture_stdout(|| {
        for &value in &values {
            unsafe { rust_driver(value) };
        }
    });

    assert_eq!(c_output.len(), values.len() * 9);
    if c_output != rust_output {
        let mismatch = c_output
            .iter()
            .zip(&rust_output)
            .position(|(c, rust)| c != rust)
            .unwrap_or(c_output.len().min(rust_output.len()));
        panic!(
            "C and Rust output differ at byte {mismatch}; C length {}, Rust length {}",
            c_output.len(),
            rust_output.len()
        );
    }
}
