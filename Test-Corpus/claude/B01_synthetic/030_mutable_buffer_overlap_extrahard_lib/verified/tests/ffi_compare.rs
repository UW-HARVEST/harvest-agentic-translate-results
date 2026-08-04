// Integration tests comparing C and Rust shared library outputs through FFI.
// Loads both .so files via libloading and asserts byte-identical results.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::sync::Mutex;

// Serialize tests that capture stdout to avoid interleaving.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_so_path() -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("c_src");
    path.push("build");
    path.push("libdriver.so");
    path.to_string_lossy().into_owned()
}

fn rust_so_path() -> String {
    // The crate produces target/<profile>/libdriver.so. Tests run with
    // CARGO_MANIFEST_DIR set; the crate is built before tests run.
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    // Try debug first then release.
    path.push("debug");
    path.push("libdriver.so");
    if path.exists() {
        return path.to_string_lossy().into_owned();
    }
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("libdriver.so");
    path.to_string_lossy().into_owned()
}

type FmaArrayFn = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
type DriverFn = unsafe extern "C" fn(data: *const c_int, len: c_int);

/// Capture writes to fd=1 (stdout) made by `f` and return the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    // Make sure libc stdio is flushed before we redirect.
    unsafe {
        libc_fflush(std::ptr::null_mut());
    }

    // Save original fd 1.
    let saved = unsafe { libc_dup(1) };
    assert!(saved >= 0, "dup failed");

    // Make a pipe.
    let mut fds = [0i32; 2];
    let r = unsafe { libc_pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    // Redirect fd 1 to write_fd.
    let r = unsafe { libc_dup2(write_fd, 1) };
    assert!(r >= 0, "dup2 failed");
    unsafe {
        libc_close(write_fd);
    }

    f();

    // Flush the libc stdio buffer for stdout BEFORE restoring fd 1.
    unsafe {
        libc_fflush(std::ptr::null_mut());
    }

    // Restore fd 1.
    let r = unsafe { libc_dup2(saved, 1) };
    assert!(r >= 0, "restore dup2 failed");
    unsafe {
        libc_close(saved);
    }

    // Read all bytes from read_fd.
    let mut buf = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_end(&mut buf).expect("reading captured stdout");
    buf
}

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}
#[inline]
unsafe fn libc_dup(fd: c_int) -> c_int { unsafe { dup(fd) } }
#[inline]
unsafe fn libc_dup2(o: c_int, n: c_int) -> c_int { unsafe { dup2(o, n) } }
#[inline]
unsafe fn libc_pipe(p: *mut c_int) -> c_int { unsafe { pipe(p) } }
#[inline]
unsafe fn libc_close(fd: c_int) -> c_int { unsafe { close(fd) } }
#[inline]
unsafe fn libc_fflush(s: *mut std::ffi::c_void) -> c_int { unsafe { fflush(s) } }

fn run_fma(lib: &Library, mul1: &[c_int], mul2: &[c_int], add: &[c_int]) -> Vec<c_int> {
    let len = mul1.len();
    assert_eq!(mul2.len(), len);
    assert_eq!(add.len(), len);
    let mut out = vec![0 as c_int; len];
    unsafe {
        let f: Symbol<FmaArrayFn> = lib.get(b"fma_array\0").expect("fma_array symbol");
        f(
            out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len as c_int,
        );
    }
    out
}

fn run_driver_capture(lib: &Library, data: &[c_int]) -> Vec<u8> {
    let _g = STDOUT_LOCK.lock().unwrap();
    capture_stdout(|| unsafe {
        let f: Symbol<DriverFn> = lib.get(b"driver\0").expect("driver symbol");
        f(data.as_ptr(), data.len() as c_int);
    })
}

#[test]
fn fma_array_matches_c() {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust so") };

    let cases: Vec<(Vec<c_int>, Vec<c_int>, Vec<c_int>)> = vec![
        (vec![], vec![], vec![]),
        (vec![0], vec![0], vec![0]),
        (vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]),
        (vec![-1, -2, -3, -4], vec![5, -6, 7, -8], vec![100, -200, 300, -400]),
        (vec![i32::MAX, i32::MIN, 0, 1], vec![2, 2, 2, 2], vec![0, 0, 0, 0]),
        (vec![i32::MAX, i32::MIN], vec![i32::MAX, i32::MIN], vec![0, 0]),
        (
            (0..100).collect(),
            (0..100).map(|x| x * 3 - 50).collect(),
            (0..100).map(|x| -x).collect(),
        ),
    ];

    for (i, (m1, m2, a)) in cases.iter().enumerate() {
        let c_out = run_fma(&c_lib, m1, m2, a);
        let r_out = run_fma(&r_lib, m1, m2, a);
        assert_eq!(c_out, r_out, "fma_array mismatch on case {i}");
    }
}

#[test]
fn driver_matches_c() {
    let c_lib = unsafe { Library::new(c_so_path()).expect("load C so") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust so") };

    let cases: Vec<Vec<c_int>> = vec![
        vec![0],
        vec![1, 2, 3],
        vec![-1, -2, 3, 4, -5],
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        vec![i32::MAX, i32::MIN, 0, 1, -1],
        (0..50).collect(),
    ];

    for (i, data) in cases.iter().enumerate() {
        let c_out = run_driver_capture(&c_lib, data);
        let r_out = run_driver_capture(&r_lib, data);
        assert_eq!(
            c_out, r_out,
            "driver stdout mismatch on case {i}\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
