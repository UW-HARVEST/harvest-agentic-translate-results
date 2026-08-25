use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("resolve test executable");
    test_executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable under target/<profile>/deps")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn randomized_inputs() -> Vec<c_int> {
    let mut inputs = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;

    for _ in 0..10_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32 as c_int);
    }

    inputs
}

fn capture_driver_output(driver: Driver, inputs: &[c_int], label: &str) -> Vec<u8> {
    let output_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{label}.out",
        std::process::id()
    ));
    let mut output = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&output_path)
        .expect("create native stdout capture");

    let stdout_fd = 1;
    unsafe {
        assert_eq!(fflush(null_mut()), 0, "flush stdout before redirect");
    }
    let saved_stdout = unsafe { dup(stdout_fd) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(output.as_raw_fd(), stdout_fd) },
        stdout_fd,
        "redirect stdout"
    );

    for &input in inputs {
        unsafe { driver(input) };
    }

    unsafe {
        assert_eq!(fflush(null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, stdout_fd), stdout_fd, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    output.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read capture");
    drop(output);
    remove_file(output_path).expect("remove native stdout capture");
    bytes
}

#[test]
fn config_1_driver_matches_for_full_int_domain_samples() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0") }.expect("load C driver");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver");
    let inputs = randomized_inputs();

    let c_output = capture_driver_output(*c_driver, &inputs, "c");
    let rust_output = capture_driver_output(*rust_driver, &inputs, "rust");

    assert_eq!(
        c_output,
        rust_output,
        "native stdout differed across {} inputs",
        inputs.len()
    );
}
