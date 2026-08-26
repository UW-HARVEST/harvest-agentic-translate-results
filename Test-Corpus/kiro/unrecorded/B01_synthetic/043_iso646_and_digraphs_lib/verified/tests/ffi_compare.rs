use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut u8) -> c_int;
    static stdout: *mut u8;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe { fflush(stdout); }

    let mut pipes = [0i32; 2];
    unsafe { pipe(pipes.as_mut_ptr()); }
    let old_stdout = unsafe { dup(1) };
    unsafe { dup2(pipes[1], 1); }
    unsafe { close(pipes[1]); }

    f();

    unsafe { fflush(stdout); }
    unsafe { dup2(old_stdout, 1); }
    unsafe { close(old_stdout); }

    let mut buf = String::new();
    let mut read_end = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    read_end.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("load C lib") }
}

fn rust_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(path).expect("load Rust lib") }
}

type DriverFn = unsafe extern "C" fn(c_int, c_int);

#[test]
fn test_driver_outputs_match() {
    let c = c_lib();
    let r = rust_lib();
    let c_driver: Symbol<DriverFn> = unsafe { c.get(b"driver").unwrap() };
    let r_driver: Symbol<DriverFn> = unsafe { r.get(b"driver").unwrap() };

    let cases: &[(c_int, c_int)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (12345, 67890),
        (-12345, -67890),
        (0x5555_5555, 0x0AAA_AAAAu32 as i32),
    ];

    for &(x, y) in cases {
        let c_out = capture_stdout(|| unsafe { c_driver(x, y) });
        let r_out = capture_stdout(|| unsafe { r_driver(x, y) });
        assert_eq!(c_out, r_out, "mismatch for driver({x}, {y}): C={c_out:?} Rust={r_out:?}");
    }
}
