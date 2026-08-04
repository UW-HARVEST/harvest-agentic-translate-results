use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    // Find the Rust .so in target/debug
    let dir = format!("{}/target/debug", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libdriver") && name.ends_with(".so") {
                return p.to_string_lossy().into_owned();
            }
        }
    }
    panic!("Rust .so not found in {}", dir);
}

type FmaArrayFn = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type DriverFn = unsafe extern "C" fn(*const c_int, c_int);

#[test]
fn test_fma_array_basic() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fma: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let r_fma: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();

        let mul1 = [1, 2, 3, 4, 5];
        let mul2 = [10, 20, 30, 40, 50];
        let add = [100, 200, 300, 400, 500];
        let mut c_out = [0i32; 5];
        let mut r_out = [0i32; 5];

        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        r_fma(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        assert_eq!(c_out, r_out, "fma_array basic mismatch");
    }
}

#[test]
fn test_fma_array_empty() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fma: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let r_fma: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();

        let mut c_out = [0i32; 0];
        let mut r_out = [0i32; 0];
        c_fma(c_out.as_mut_ptr(), c_out.as_ptr(), c_out.as_ptr(), c_out.as_ptr(), 0);
        r_fma(r_out.as_mut_ptr(), r_out.as_ptr(), r_out.as_ptr(), r_out.as_ptr(), 0);
        assert_eq!(c_out, r_out, "fma_array empty mismatch");
    }
}

#[test]
fn test_fma_array_negative() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fma: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let r_fma: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();

        let mul1 = [-1, -2, 3];
        let mul2 = [4, -5, -6];
        let add = [7, 8, -9];
        let mut c_out = [0i32; 3];
        let mut r_out = [0i32; 3];

        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        r_fma(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        assert_eq!(c_out, r_out, "fma_array negative mismatch");
    }
}

#[test]
fn test_fma_array_inplace() {
    // C code calls fma_array(out, out, out, out, len) in inner(), so test aliased pointers
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fma: Symbol<FmaArrayFn> = c_lib.get(b"fma_array").unwrap();
        let r_fma: Symbol<FmaArrayFn> = r_lib.get(b"fma_array").unwrap();

        let mut c_buf = [2, 3, 4];
        let mut r_buf = [2, 3, 4];

        c_fma(c_buf.as_mut_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), 3);
        r_fma(r_buf.as_mut_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), 3);
        assert_eq!(c_buf, r_buf, "fma_array inplace mismatch");
    }
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Flush existing stdout
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipefd = [0i32; 2];
    unsafe { libc::pipe(pipefd.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipefd[1], 1) };

    f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipefd[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipefd[0]) };
    // Set non-blocking and read available data
    unsafe {
        libc::fcntl(pipefd[0], libc::F_SETFL, libc::O_NONBLOCK);
    }
    let _ = reader.read_to_string(&mut buf);
    buf
}

use std::os::unix::io::FromRawFd;

#[test]
fn test_driver_output() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let data = [2, 3, 4];

        let c_out = capture_stdout(|| { c_driver(data.as_ptr(), 3); });
        let r_out = capture_stdout(|| { r_driver(data.as_ptr(), 3); });
        assert_eq!(c_out, r_out, "driver output mismatch");
    }
}

#[test]
fn test_driver_single() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let data = [7];
        let c_out = capture_stdout(|| { c_driver(data.as_ptr(), 1); });
        let r_out = capture_stdout(|| { r_driver(data.as_ptr(), 1); });
        assert_eq!(c_out, r_out, "driver single mismatch");
    }
}

#[test]
fn test_driver_empty() {
    unsafe {
        let c_lib = Library::new(C_LIB).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_driver: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_driver: Symbol<DriverFn> = r_lib.get(b"driver").unwrap();

        let data: [i32; 0] = [];
        let c_out = capture_stdout(|| { c_driver(data.as_ptr(), 0); });
        let r_out = capture_stdout(|| { r_driver(data.as_ptr(), 0); });
        assert_eq!(c_out, r_out, "driver empty mismatch");
    }
}
