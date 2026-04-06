use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// Capture stdout produced by `f()` by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);
        let mut file = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn c_lib() -> Library {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load C .so") }
}

#[test]
fn test_run() {
    let lib = c_lib();
    // Test run with a known house and extra_bedrooms value
    let cases: Vec<(c_int, c_int, f64, c_int)> = vec![
        (2, 5, 2.5, 3),
        (0, 0, 0.0, 0),
        (1, 1, 1.0, 10),
    ];
    for (floors, bedrooms, bathrooms, extra) in cases {
        // C version
        let c_out = capture_stdout(|| unsafe {
            let c_run: Symbol<unsafe extern "C" fn(*mut HouseT, c_int)> =
                lib.get(b"run").unwrap();
            let mut house = HouseT { floors, bedrooms, bathrooms };
            c_run(&mut house, extra);
        });
        // Rust version
        let rust_out = capture_stdout(|| {
            let mut house = driver::HouseT {
                floors,
                bedrooms,
                bathrooms,
            };
            driver::run(&mut house, extra);
        });
        assert_eq!(c_out, rust_out, "run mismatch for ({floors},{bedrooms},{bathrooms},{extra})");
    }
}

#[test]
fn test_driver_valid() {
    let lib = c_lib();
    let inputs = ["3", "0", "-5", "100"];
    for input in &inputs {
        let cs = CString::new(*input).unwrap();
        let c_out = capture_stdout(|| unsafe {
            let c_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"driver").unwrap();
            c_driver(cs.as_ptr());
        });
        let rust_out = capture_stdout(|| {
            let cs2 = CString::new(*input).unwrap();
            driver::driver(cs2.as_ptr());
        });
        assert_eq!(c_out, rust_out, "driver mismatch for input '{input}'");
    }
}

#[test]
fn test_driver_invalid() {
    let lib = c_lib();
    let inputs = ["abc", "", "99999999999999999999"];
    for input in &inputs {
        let cs = CString::new(*input).unwrap();
        let c_out = capture_stdout(|| unsafe {
            let c_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"driver").unwrap();
            c_driver(cs.as_ptr());
        });
        let rust_out = capture_stdout(|| {
            let cs2 = CString::new(*input).unwrap();
            driver::driver(cs2.as_ptr());
        });
        assert_eq!(c_out, rust_out, "driver mismatch for invalid input '{input}'");
    }
}
