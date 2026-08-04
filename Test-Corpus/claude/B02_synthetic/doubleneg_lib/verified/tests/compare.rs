// Integration tests that load BOTH the C .so and the Rust .so via libloading
// and compare their outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Mutex;

// Global mutex used to serialize tests that touch stdout, so concurrent tests
// don't clobber each other's pipe redirection.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO: &str = "target/release/libdoubleneg_lib.so";

fn load_libs() -> (Library, Library) {
    let c_lib = unsafe { Library::new(C_SO).expect("Failed to load C SO") };
    let r_lib = unsafe { Library::new(RUST_SO).expect("Failed to load Rust SO") };
    (c_lib, r_lib)
}

// Helper: fork a child process to run the function in isolation, capture its stdout.
// This avoids interference from the test runner's own writes to stdout (e.g. "test ... ok").
fn capture_stdout_in_child<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        // Flush parent stdout first so any pending parent output isn't duplicated by fork.
        libc::fflush(std::ptr::null_mut());

        // Create pipe
        let mut fds: [c_int; 2] = [0; 2];
        let r = libc::pipe(fds.as_mut_ptr());
        assert_eq!(r, 0, "pipe failed");
        let read_fd = fds[0];
        let write_fd = fds[1];

        let pid = libc::fork();
        if pid < 0 {
            panic!("fork failed");
        }
        if pid == 0 {
            // Child: close read end, redirect stdout to write end, run the closure, exit.
            libc::close(read_fd);
            libc::dup2(write_fd, 1);
            libc::close(write_fd);
            f();
            libc::fflush(std::ptr::null_mut());
            // Exit without running test runner cleanup.
            libc::_exit(0);
        }

        // Parent: close write end, read from pipe, wait for child.
        libc::close(write_fd);
        let mut file = std::fs::File::from_raw_fd(read_fd);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read pipe");
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        buf
    }
}

#[test]
fn test_process_negation() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { c_lib.get(b"process_negation").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { r_lib.get(b"process_negation").unwrap() };

    let inputs: [c_int; 9] = [0, 1, -1, 100, -100, c_int::MAX, c_int::MIN, 42, -42];
    for &v in &inputs {
        let c_out = unsafe { c_fn(v) };
        let r_out = unsafe { r_fn(v) };
        assert_eq!(c_out, r_out, "process_negation({}) mismatch: C={}, R={}", v, c_out, r_out);
    }
}

#[test]
fn test_convert_double_to_int() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { c_lib.get(b"convert_double_to_int").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { r_lib.get(b"convert_double_to_int").unwrap() };

    let inputs = [
        0.0_f64,
        1.0,
        -1.0,
        42.7,
        -42.7,
        1e9,
        -1e9,
        1e15,
        -1e15,
        2147483647.0,        // INT_MAX
        2147483648.0,        // INT_MAX + 1 -> overflow -> INT_MIN on x86_64
        -2147483648.0,       // INT_MIN
        -2147483649.0,       // INT_MIN - 1 -> overflow -> INT_MIN
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for &v in &inputs {
        let c_out = unsafe { c_fn(v) };
        let r_out = unsafe { r_fn(v) };
        assert_eq!(c_out, r_out, "convert_double_to_int({}) mismatch: C={}, R={}", v, c_out, r_out);
    }
}

#[test]
fn test_create_numeric_buffer() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
        unsafe { c_lib.get(b"create_numeric_buffer").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
        unsafe { r_lib.get(b"create_numeric_buffer").unwrap() };

    let seeds = [0_i32, 1, -1, 42, 100, 256, -256, 1000, -1000, 12345, c_int::MAX, c_int::MIN];
    let sizes = [0_i32, 1, 16, 64, 256, 300];
    for &size in &sizes {
        for &seed in &seeds {
            if size <= 0 {
                continue;
            }
            let mut c_buf = vec![0_i8; size as usize];
            let mut r_buf = vec![0_i8; size as usize];
            unsafe {
                c_fn(c_buf.as_mut_ptr(), size, seed);
                r_fn(r_buf.as_mut_ptr(), size, seed);
            }
            assert_eq!(c_buf, r_buf, "create_numeric_buffer(size={}, seed={}) mismatch", size, seed);
        }
    }
}

#[test]
fn test_find_value_in_buffer() {
    let (c_lib, r_lib) = load_libs();
    let c_create: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
        unsafe { c_lib.get(b"create_numeric_buffer").unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int> =
        unsafe { c_lib.get(b"find_value_in_buffer").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int> =
        unsafe { r_lib.get(b"find_value_in_buffer").unwrap() };

    // Create deterministic buffer
    let size: c_int = 256;
    let mut buf = vec![0_i8; size as usize];
    unsafe { c_create(buf.as_mut_ptr(), size, 42); }

    let search_vals = [
        0_i32, 1, -1, 42, 100, 127, 128, 200, 255, 256, 257, -128, -129, 500, -500,
        c_int::MAX, c_int::MIN,
    ];
    for &v in &search_vals {
        let c_out = unsafe { c_fn(buf.as_ptr(), size as usize, v) };
        let r_out = unsafe { r_fn(buf.as_ptr(), size as usize, v) };
        assert_eq!(c_out, r_out, "find_value_in_buffer(.., {}) mismatch: C={}, R={}", v, c_out, r_out);
    }

    // Also test with empty
    let c_out = unsafe { c_fn(buf.as_ptr(), 0, 42) };
    let r_out = unsafe { r_fn(buf.as_ptr(), 0, 42) };
    assert_eq!(c_out, r_out, "empty buffer search mismatch");
}

#[test]
fn test_calculate_with_doubles() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> f64> =
        unsafe { c_lib.get(b"calculate_with_doubles").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> f64> =
        unsafe { r_lib.get(b"calculate_with_doubles").unwrap() };

    let cases = [
        (0_i32, 1_i32, 0_i32),
        (1, 1, 0),
        (10, 2, 3),
        (100, 5, 7),
        (1, 0, 5),     // b == 0
        (-100, 7, -3),
        (1234, -56, 9),
        (i32::MAX, 1, 0),
        (i32::MIN, 1, 0),
        (1, 3, 5),
        (1, 7, 12),
        (-1, -1, -1),
    ];
    for &(a, b, c) in &cases {
        let c_out = unsafe { c_fn(a, b, c) };
        let r_out = unsafe { r_fn(a, b, c) };
        // Use bit-pattern equality to handle NaN equivalence (none expected here, but be safe).
        assert_eq!(c_out.to_bits(), r_out.to_bits(),
            "calculate_with_doubles({}, {}, {}) mismatch: C={}, R={}", a, b, c, c_out, r_out);
    }
}

#[test]
fn test_doubleneg() {
    let (c_lib, r_lib) = load_libs();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { c_lib.get(b"doubleneg").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { r_lib.get(b"doubleneg").unwrap() };

    let cases = [
        (0_i32, 0_i32, 0_i32, 0_i32),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (10, 0, 0, 0),
        (5, 7, 11, 13),
        (-100, 50, -25, 12),
        (123456, 789, 0, 1),
    ];
    let _g = STDOUT_LOCK.lock().unwrap();
    for &(p1, p2, p3, p4) in &cases {
        let c_buf = capture_stdout_in_child(|| {
            let _ = unsafe { c_fn(p1, p2, p3, p4) };
        });
        let r_buf = capture_stdout_in_child(|| {
            let _ = unsafe { r_fn(p1, p2, p3, p4) };
        });

        // Also collect return value separately to compare
        let c_ret = unsafe { c_fn(p1, p2, p3, p4) };
        let r_ret = unsafe { r_fn(p1, p2, p3, p4) };
        assert_eq!(c_ret, r_ret,
            "doubleneg({},{},{},{}) return mismatch: C={}, R={}", p1, p2, p3, p4, c_ret, r_ret);

        if c_buf != r_buf {
            let c_str = String::from_utf8_lossy(&c_buf);
            let r_str = String::from_utf8_lossy(&r_buf);
            panic!("doubleneg({},{},{},{}) stdout mismatch:\n--- C ---\n{}\n--- R ---\n{}",
                p1, p2, p3, p4, c_str, r_str);
        }
    }
}
