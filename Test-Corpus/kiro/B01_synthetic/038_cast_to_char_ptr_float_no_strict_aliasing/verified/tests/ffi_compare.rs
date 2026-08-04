use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout output from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush Rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1) };

    f();

    // flush both C and Rust stdout
    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };
    unsafe { libc::close(fds[1]) };

    let mut buf = String::new();
    let mut read_end = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    read_end.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/libdriver_c.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target/release/libdriver.so")
}

#[test]
fn test_driver_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let rs_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let test_values: &[f32] = &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        1.23456789,
        -42.0,
        std::f32::consts::PI,
        f32::from_bits(0x7FC00001), // signaling NaN variant
        f32::from_bits(0xFFFFFFFF), // negative NaN
    ];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(f32)> = c_lib.get(b"driver").unwrap();
            f(val);
        });
        let rs_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(f32)> = rs_lib.get(b"driver").unwrap();
            f(val);
        });
        assert_eq!(
            c_out, rs_out,
            "Mismatch for input {val} (bits {:08x}): C={c_out:?} Rust={rs_out:?}",
            val.to_bits()
        );
    }
}
