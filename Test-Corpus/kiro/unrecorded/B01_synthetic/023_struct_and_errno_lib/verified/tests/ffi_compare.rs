use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::os::unix::io::FromRawFd;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    // flush rust stdout first
    use std::io::Write;
    std::io::stdout().flush().unwrap();

    let mut pipes = [0i32; 2];
    unsafe { libc::pipe(pipes.as_mut_ptr()) };
    let (read_fd, write_fd) = (pipes[0], pipes[1]);

    // save original stdout
    let orig = unsafe { libc::dup(1) };
    // redirect stdout to pipe write end
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };

    f();

    // flush C stdout (printf) and Rust stdout (println)
    std::io::stdout().flush().unwrap();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    // restore original stdout
    unsafe { libc::dup2(orig, 1) };
    unsafe { libc::close(orig) };

    // read captured output
    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    file.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib() -> Library {
    unsafe {
        Library::new("/tmp/harvest-work-lCHwzp/translated_rust/c_src/build/libdriver.so").unwrap()
    }
}

fn rust_lib() -> Library {
    unsafe {
        Library::new("/tmp/harvest-work-lCHwzp/translated_rust/target/debug/libdriver.so").unwrap()
    }
}

#[test]
fn test_run_basic() {
    let c = c_lib();
    let r = rust_lib();

    let c_run: Symbol<unsafe extern "C" fn(*mut House, c_int)> =
        unsafe { c.get(b"run").unwrap() };
    let r_run: Symbol<unsafe extern "C" fn(*mut House, c_int)> =
        unsafe { r.get(b"run").unwrap() };

    // Test with several extra_bedrooms values
    for extra in &[0, 1, 3, -1, 100] {
        let mut c_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };
        let mut r_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };

        let c_out = capture_stdout(|| unsafe { c_run(&mut c_house, *extra) });
        let r_out = capture_stdout(|| unsafe { r_run(&mut r_house, *extra) });

        assert_eq!(c_out, r_out, "run() mismatch for extra_bedrooms={extra}");
        // Also verify struct state matches
        assert_eq!(c_house.floors, r_house.floors, "floors mismatch after run(extra={extra})");
        assert_eq!(c_house.bedrooms, r_house.bedrooms, "bedrooms mismatch after run(extra={extra})");
        assert_eq!(c_house.bathrooms, r_house.bathrooms, "bathrooms mismatch after run(extra={extra})");
    }
}

#[test]
fn test_run_called_twice() {
    // Mirrors what driver() does: calls run twice on the same house
    let c = c_lib();
    let r = rust_lib();

    let c_run: Symbol<unsafe extern "C" fn(*mut House, c_int)> =
        unsafe { c.get(b"run").unwrap() };
    let r_run: Symbol<unsafe extern "C" fn(*mut House, c_int)> =
        unsafe { r.get(b"run").unwrap() };

    let mut c_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };
    let mut r_house = House { floors: 2, bedrooms: 5, bathrooms: 2.5 };

    let c_out = capture_stdout(|| unsafe {
        c_run(&mut c_house, 3);
        c_run(&mut c_house, 3);
    });
    let r_out = capture_stdout(|| unsafe {
        r_run(&mut r_house, 3);
        r_run(&mut r_house, 3);
    });

    assert_eq!(c_out, r_out, "run() called twice mismatch");
}

#[test]
fn test_driver_valid_input() {
    let c = c_lib();
    let r = rust_lib();

    let c_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c.get(b"driver").unwrap() };
    let r_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r.get(b"driver").unwrap() };

    for input in &["3", "0", "-1", "100", "  42", "+7"] {
        let cs = CString::new(*input).unwrap();
        let c_out = capture_stdout(|| unsafe { c_driver(cs.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_driver(cs.as_ptr()) });
        assert_eq!(c_out, r_out, "driver() mismatch for input={input:?}");
    }
}

#[test]
fn test_driver_invalid_input() {
    let c = c_lib();
    let r = rust_lib();

    let c_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c.get(b"driver").unwrap() };
    let r_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r.get(b"driver").unwrap() };

    for input in &["abc", "", "  ", "xyz123"] {
        let cs = CString::new(*input).unwrap();
        let c_out = capture_stdout(|| unsafe { c_driver(cs.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_driver(cs.as_ptr()) });
        assert_eq!(c_out, r_out, "driver() mismatch for invalid input={input:?}");
    }
}

#[test]
fn test_driver_edge_cases() {
    let c = c_lib();
    let r = rust_lib();

    let c_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c.get(b"driver").unwrap() };
    let r_driver: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { r.get(b"driver").unwrap() };

    // Trailing non-digit chars: C strtol parses "123abc" as 123
    for input in &["123abc", "0x10", "2147483647", "-2147483648", "999999999999999999999"] {
        let cs = CString::new(*input).unwrap();
        let c_out = capture_stdout(|| unsafe { c_driver(cs.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_driver(cs.as_ptr()) });
        assert_eq!(c_out, r_out, "driver() mismatch for edge input={input:?}");
    }
}
