// Integration test that loads both C and Rust shared libraries via libloading
// and compares their behavior byte-for-byte through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    // Use the same profile as cargo test (debug)
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libgotomach_lib.so")
}

type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

/// Capture stdout while running `f`. Uses dup/dup2 to redirect FD 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    unsafe {
        // Flush C stdout to ensure we capture only what `f` writes.
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        let mut fds = [0i32; 2];
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            panic!("pipe failed");
        }
        let read_fd = fds[0];
        let write_fd = fds[1];

        // Set the write end of the pipe to be FD 1.
        if libc::dup2(write_fd, 1) < 0 {
            panic!("dup2 failed");
        }
        libc::close(write_fd);

        // Run the function while stdout points at the pipe.
        f();

        // Flush so anything buffered ends up in the pipe.
        libc::fflush(std::ptr::null_mut());

        // Restore stdout
        libc::dup2(saved, 1);
        libc::close(saved);

        // Read everything from the pipe (it should now be closed at the write end).
        let mut buf = Vec::new();
        use std::os::unix::io::FromRawFd;
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let _ = file.read_to_end(&mut buf);
        buf
    }
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("failed to load C .so");
        let rust_lib = Library::new(rust_so_path()).expect("failed to load Rust .so");
        (c_lib, rust_lib)
    }
}

fn run_op_pair(name: &[u8]) {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<OpFn> = c_lib.get(name).expect("C op symbol missing");
        let rust_fn: Symbol<OpFn> = rust_lib.get(name).expect("Rust op symbol missing");

        let cases: &[c_int] = &[0, 1, -1, 2, 5, 10, 100, -100, 1000, 32767, -32768, i32::MAX / 4, i32::MIN / 4];
        for &v in cases {
            let c_out = c_fn(v, 12345, std::ptr::null_mut());
            let r_out = rust_fn(v, 12345, std::ptr::null_mut());
            assert_eq!(
                c_out, r_out,
                "op {} differed for value {}: c={} rust={}",
                std::str::from_utf8(name).unwrap(),
                v,
                c_out,
                r_out
            );
        }
    }
}

#[test]
fn process_value_matches() {
    run_op_pair(b"process_value");
}

#[test]
fn double_value_matches() {
    run_op_pair(b"double_value");
}

#[test]
fn triple_value_matches() {
    run_op_pair(b"triple_value");
}

fn run_gotomach_case(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<GotomachFn> = c_lib.get(b"gotomach").expect("C gotomach missing");
        let rust_fn: Symbol<GotomachFn> = rust_lib.get(b"gotomach").expect("Rust gotomach missing");

        let mut c_out: c_int = 0;
        let c_stdout = capture_stdout(|| {
            c_out = c_fn(iterations, seed, mode, threshold);
        });

        let mut r_out: c_int = 0;
        let r_stdout = capture_stdout(|| {
            r_out = rust_fn(iterations, seed, mode, threshold);
        });

        assert_eq!(
            c_out, r_out,
            "gotomach return differed for ({}, {}, {}, {}): c={} rust={}",
            iterations, seed, mode, threshold, c_out, r_out
        );
        assert_eq!(
            c_stdout, r_stdout,
            "gotomach stdout differed for ({}, {}, {}, {}):\nC:    {}\nRust: {}",
            iterations,
            seed,
            mode,
            threshold,
            String::from_utf8_lossy(&c_stdout),
            String::from_utf8_lossy(&r_stdout)
        );
    }
}

#[test]
fn gotomach_invalid_iterations_negative() {
    run_gotomach_case(-1, 0, 0, 100);
}

#[test]
fn gotomach_invalid_iterations_too_large() {
    run_gotomach_case(70000, 0, 0, 100);
}

#[test]
fn gotomach_invalid_seed_negative() {
    run_gotomach_case(10, -5, 0, 100);
}

#[test]
fn gotomach_invalid_seed_too_large() {
    run_gotomach_case(10, 70000, 0, 100);
}

#[test]
fn gotomach_zero_iterations() {
    run_gotomach_case(0, 0, 0, 100);
}

#[test]
fn gotomach_mode_0_basic() {
    run_gotomach_case(10, 5, 0, 100);
}

#[test]
fn gotomach_mode_1_basic() {
    run_gotomach_case(10, 5, 1, 100);
}

#[test]
fn gotomach_mode_2_basic() {
    run_gotomach_case(10, 5, 2, 100);
}

#[test]
fn gotomach_mode_invalid_default() {
    run_gotomach_case(10, 5, 99, 100);
}

#[test]
fn gotomach_high_threshold() {
    run_gotomach_case(50, 7, 1, 1_000_000);
}

#[test]
fn gotomach_low_threshold() {
    run_gotomach_case(50, 7, 1, -1_000_000);
}

#[test]
fn gotomach_mode_2_large() {
    run_gotomach_case(1000, 13, 2, 50_000);
}

#[test]
fn gotomach_mode_0_large() {
    run_gotomach_case(2000, 99, 0, 5000);
}

#[test]
fn gotomach_zero_seed_mode_2() {
    run_gotomach_case(100, 0, 2, 100);
}

#[test]
fn gotomach_max_iterations() {
    run_gotomach_case(65535, 1, 0, 50);
}
