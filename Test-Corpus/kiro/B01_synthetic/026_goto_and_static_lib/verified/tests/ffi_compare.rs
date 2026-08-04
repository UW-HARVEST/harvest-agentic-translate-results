use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

/// Capture stdout produced by `f()` by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // flush before redirect
    unsafe { libc::fflush(std::ptr::null_mut()); }
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let old_stdout = unsafe { libc::dup(1) };
    assert!(old_stdout >= 0);
    unsafe { libc::dup2(fds[1], 1); }
    unsafe { libc::close(fds[1]); }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    unsafe { libc::dup2(old_stdout, 1); }
    unsafe { libc::close(old_stdout); }

    let mut buf = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_end(&mut buf).unwrap();
    buf
}

fn call_driver(lib: &Library, x: c_int, y: c_int, z: c_int) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let func: Symbol<unsafe extern "C" fn(c_int, c_int, c_int)> =
            lib.get(b"driver").unwrap();
        func(x, y, z);
    })
}

#[test]
fn driver_outputs_match() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rs_lib = unsafe { Library::new(RUST_LIB).expect("load Rust lib") };

    let cases: &[(c_int, c_int, c_int)] = &[
        (1, 2, 3),   // all conditions pass -> "Ok!\n" path
        (0, 2, 3),   // x != 1 -> fail
        (1, 0, 3),   // y != 2 -> fail
        (1, 2, 0),   // z != 3 -> fail
        (0, 0, 0),   // all wrong
        (-1, -2, -3),
        (1, 2, 4),
        (2, 2, 3),
    ];

    for &(x, y, z) in cases {
        let c_out = call_driver(&c_lib, x, y, z);
        let rs_out = call_driver(&rs_lib, x, y, z);
        assert_eq!(
            c_out, rs_out,
            "Mismatch for driver({x}, {y}, {z}):\n  C:    {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rs_out),
        );
    }
}
