use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

type Driver = unsafe extern "C" fn(c_float);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libdriver.so"),
        root.join("target/debug/libdriver.so"),
    )
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }.unwrap_or_else(|error| {
        panic!("failed to load {}: {error}", path.display());
    })
}

fn float_cases() -> Vec<f32> {
    let edge_bits = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0x7f7f_ffff,
        0x8080_0000,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7f80_0001,
        0x7fc0_0000,
        0x7fff_ffff,
        0xff80_0001,
        0xffc0_0000,
        0xffff_ffff,
    ];

    let mut cases: Vec<f32> = edge_bits.into_iter().map(f32::from_bits).collect();
    let mut state = 0xd1ff_3a5e_u32;
    for _ in 0..16_384 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        cases.push(f32::from_bits(state));
    }
    cases
}

fn capture_driver_output(driver: Driver, cases: &[f32]) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock was poisoned");
    let mut pipe_fds = [-1; 2];
    let saved_stdout;

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);

        saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], 1), 1);
        assert_eq!(close(pipe_fds[1]), 0);
    }

    let read_fd = pipe_fds[0];
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = unsafe { read(read_fd, buffer.as_mut_ptr().cast(), buffer.len()) };
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        unsafe {
            assert_eq!(close(read_fd), 0);
        }
        output
    });

    unsafe {
        for &case in cases {
            driver(case);
        }

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
    }

    reader.join().expect("stdout reader thread panicked")
}

#[test]
fn configs_row_1_driver_all_float_shapes() {
    let (c_path, rust_path) = library_paths();
    let c_library = load_library(&c_path);
    let rust_library = load_library(&rust_path);
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("C driver symbol");
    let rust_driver: Symbol<'_, Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("Rust driver symbol");
    let cases = float_cases();

    let c_output = capture_driver_output(*c_driver, &cases);
    let rust_output = capture_driver_output(*rust_driver, &cases);

    assert_eq!(c_output.len(), cases.len() * 9);
    assert_eq!(rust_output, c_output);
}
