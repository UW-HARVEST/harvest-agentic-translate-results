use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(f64);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    manifest_dir()
        .join("target")
        .join("release")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("..")
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
}

fn capture_driver_output(driver: Driver, inputs: &[f64]) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe");
    }

    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut file = unsafe { File::from_raw_fd(pipe_fds[0]) };
        file.read_to_end(&mut bytes).expect("read captured stdout");
        bytes
    });

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup stdout");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "redirect stdout");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close copied pipe writer");

    for &input in inputs {
        unsafe { driver(input) };
    }

    assert_eq!(
        unsafe { fflush(std::ptr::null_mut()) },
        0,
        "fflush captured output"
    );
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout");
    assert_eq!(unsafe { close(saved_stdout) }, 0, "close saved stdout");

    reader.join().expect("stdout reader thread panicked")
}

fn binary64_inputs() -> Vec<f64> {
    let edge_bits = [
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        0x800f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x8010_0000_0000_0000,
        0x3fd3_3333_3333_3333,
        0xbfd3_3333_3333_3333,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0001,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
    ];
    let mut inputs: Vec<_> = edge_bits.into_iter().map(f64::from_bits).collect();

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..25_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(f64::from_bits(state));
    }

    inputs
}

fn assert_byte_identical(inputs: &[f64], c_output: &[u8], rust_output: &[u8]) {
    if c_output == rust_output {
        return;
    }

    let differing_byte = c_output
        .iter()
        .zip(rust_output)
        .position(|(c, rust)| c != rust)
        .unwrap_or_else(|| c_output.len().min(rust_output.len()));
    let differing_call = c_output[..differing_byte.min(c_output.len())]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count();
    let input = inputs.get(differing_call).map(|value| value.to_bits());

    panic!(
        "output differs at byte {differing_byte}, call {differing_call}, \
         input bits {input:#018x?}; C length {}, Rust length {}",
        c_output.len(),
        rust_output.len()
    );
}

#[test]
fn driver_matches_for_binary64_input_space() {
    let c_library = load_library(&c_library_path());
    let rust_library = load_library(&rust_library_path());
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0") }.expect("C driver");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("Rust driver");
    let inputs = binary64_inputs();

    let c_output = capture_driver_output(*c_driver, &inputs);
    let rust_output = capture_driver_output(*rust_driver, &inputs);

    assert_byte_identical(&inputs, &c_output, &rust_output);
}
