use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn fflush(stream: *mut libc::c_void) -> libc::c_int;
    static stdout: *mut libc::c_void;
}

fn capture_stdout(f: impl FnOnce()) -> String {
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    unsafe { fflush(stdout); }

    let (read_fd, write_fd) = unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        (fds[0], fds[1])
    };

    let orig_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); }

    f();

    std::io::stdout().flush().unwrap();
    unsafe {
        fflush(stdout);
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);
        libc::close(write_fd);
    }

    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    let c_driver: Symbol<unsafe extern "C" fn(f32)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver symbol") };

    let test_values: &[f32] = &[
        0.0, -0.0, 1.0, -1.0, 3.14,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN,
        f32::MIN, f32::MAX, f32::MIN_POSITIVE,
        1.23456789e10, 1.23456789e-10,
    ];

    for &val in test_values {
        let c_output = capture_stdout(|| unsafe { c_driver(val) });
        let rust_output = capture_stdout(|| driver::driver(val));
        assert_eq!(
            c_output, rust_output,
            "Mismatch for input {val}: C={c_output:?} Rust={rust_output:?}"
        );
    }
}
