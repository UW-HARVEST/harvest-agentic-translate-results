use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::c_char;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // Flush before capturing
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipes[1], 1) };
    unsafe { libc::close(pipes[1]) };

    f();

    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipes[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src/build/libdriver.so"),
        )
        .expect("Failed to load C .so")
    }
}

fn rust_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

#[test]
fn test_print_int_line() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { c.get(b"printIntLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32)> =
        unsafe { r.get(b"printIntLine").unwrap() };

    for val in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(val) });
        let r_out = capture_stdout(|| unsafe { r_fn(val) });
        assert_eq!(c_out, r_out, "printIntLine mismatch for {val}");
    }
}

#[test]
fn test_print_line() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c.get(b"printLine").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r.get(b"printLine").unwrap() };

    let cases = ["hello", "", "Calling good()...", "with spaces and 123"];
    for s in cases {
        let cs = CString::new(s).unwrap();
        let c_out = capture_stdout(|| unsafe { c_fn(cs.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_fn(cs.as_ptr()) });
        assert_eq!(c_out, r_out, "printLine mismatch for {s:?}");
    }

    // Test NULL
    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture_stdout(|| unsafe { r_fn(std::ptr::null()) });
    assert_eq!(c_out, r_out, "printLine mismatch for NULL");
}

#[test]
fn test_bad() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"bad").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"bad").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "bad() output mismatch");
}

#[test]
fn test_good() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c.get(b"good").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn()> = unsafe { r.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let r_out = capture_stdout(|| unsafe { r_fn() });
    assert_eq!(c_out, r_out, "good() output mismatch");
}

#[test]
fn test_main() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, *const *const c_char) -> i32> =
        unsafe { c.get(b"main").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(i32, *const *const c_char) -> i32> =
        unsafe { r.get(b"main").unwrap() };

    let c_out = capture_stdout(|| {
        let ret = unsafe { c_fn(0, std::ptr::null()) };
        assert_eq!(ret, 0);
    });
    let r_out = capture_stdout(|| {
        let ret = unsafe { r_fn(0, std::ptr::null()) };
        assert_eq!(ret, 0);
    });
    assert_eq!(c_out, r_out, "main() output mismatch");
}
