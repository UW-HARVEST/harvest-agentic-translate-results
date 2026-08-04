use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libcleanup_lib.so");

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_LIB).expect("Failed to load C .so");
        let r = Library::new(RUST_LIB).expect("Failed to load Rust .so");
        (c, r)
    }
}

// ---- cleanup_resources (lowest level) ----

#[test]
fn test_cleanup_resources_null() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char)> =
            c_lib.get(b"cleanup_resources").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char)> =
            r_lib.get(b"cleanup_resources").unwrap();
        // Both should handle null without crashing
        c_fn(std::ptr::null_mut());
        r_fn(std::ptr::null_mut());
    }
}

#[test]
fn test_cleanup_resources_valid_ptr() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char)> =
            c_lib.get(b"cleanup_resources").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char)> =
            r_lib.get(b"cleanup_resources").unwrap();
        // Both should free without crashing
        let p1 = libc::malloc(16) as *mut c_char;
        c_fn(p1);
        let p2 = libc::malloc(16) as *mut c_char;
        r_fn(p2);
    }
}

// ---- cleanup (main function) ----

fn call_cleanup(lib: &Library, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            lib.get(b"cleanup").unwrap();
        f(a, b, c, d)
    }
}

#[test]
fn test_cleanup_default_values() {
    let (c_lib, r_lib) = load_libs();
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (10, 20, 30, 40),
        (10, 10, 10, 10),
        (20, 20, 20, 20),
        (30, 30, 30, 30),
        (40, 40, 40, 40),
        (0, 0, 0, 0),
        (10, 30, 20, 40),
        (5, 10, 15, 20),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (10, 20, 30, 5),
        (10, 0, 30, 0),
    ];
    for &(a, b, c, d) in cases {
        let c_result = call_cleanup(&c_lib, a, b, c, d);
        let r_result = call_cleanup(&r_lib, a, b, c, d);
        assert_eq!(
            c_result, r_result,
            "cleanup({a},{b},{c},{d}): C={c_result}, Rust={r_result}"
        );
    }
}

// ---- print_result (mid-level) ----

fn capture_print_result(lib: &Library, label: &[u8], result: c_int) -> Vec<u8> {
    use std::io::Read;
    use std::os::fd::FromRawFd;

    unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char, c_int)> =
            lib.get(b"print_result").unwrap();

        // Flush any pending stdout before capturing
        libc::fflush(std::ptr::null_mut());

        // Create a pipe to capture stdout
        let mut fds = [0i32; 2];
        libc::pipe(fds.as_mut_ptr());
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);

        f(label.as_ptr() as *const c_char, result);
        libc::fflush(std::ptr::null_mut());

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);

        let mut buf = Vec::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        reader.read_to_end(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_print_result() {
    let (c_lib, r_lib) = load_libs();
    let cases: &[(&[u8], c_int)] = &[
        (b"test\0", 42),
        (b"result\0", 0),
        (b"negative\0", -1),
        (b"\0", 999),
    ];
    for &(label, val) in cases {
        let c_out = capture_print_result(&c_lib, label, val);
        let r_out = capture_print_result(&r_lib, label, val);
        assert_eq!(
            c_out, r_out,
            "print_result output mismatch for val={val}: C={:?}, Rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
