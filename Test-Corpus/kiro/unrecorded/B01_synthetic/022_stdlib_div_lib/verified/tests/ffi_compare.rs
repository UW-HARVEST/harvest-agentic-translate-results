use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    // cargo test builds into target/debug/deps; the cdylib is in target/debug
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{dir}/target/debug/libdriver.so")
}

/// Capture stdout produced by `f()` using pipe + dup2.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        let mut pipefd = [0 as c_int; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(pipefd[1], 1);
        f();
        libc::fflush(std::ptr::null_mut()); // flush C stdio
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipefd[1]);
        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn call_driver(lib: &Library, x: c_int, y: c_int) -> String {
    capture_stdout(|| unsafe {
        let func: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            lib.get(b"driver").expect("symbol 'driver' not found");
        func(x, y);
    })
}

#[test]
fn driver_matches() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let cases: &[(c_int, c_int)] = &[
        (10, 3),
        (0, 1),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (100, 7),
        (1, 1),
        (i32::MAX, 1),
        (i32::MIN + 1, 2),
    ];

    for &(x, y) in cases {
        let c_out = call_driver(&c_lib, x, y);
        let r_out = call_driver(&r_lib, x, y);
        assert_eq!(c_out, r_out, "mismatch for driver({x}, {y}): C={c_out:?} Rust={r_out:?}");
    }
}
