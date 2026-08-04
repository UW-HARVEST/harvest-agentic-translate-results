use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn fdopen(fd: i32, mode: *const u8) -> *mut std::ffi::c_void;
}

static C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let p = format!("{dir}/target/debug/libdriver.so");
    assert!(std::path::Path::new(&p).exists(), "Rust .so not found");
    p
}

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        fflush(fdopen(1, b"w\0".as_ptr()));

        let mut pipefd = [0i32; 2];
        assert_eq!(pipe(pipefd.as_mut_ptr()), 0);

        let saved = dup(1);
        assert!(saved >= 0);
        dup2(pipefd[1], 1);
        close(pipefd[1]);

        f();

        fflush(fdopen(1, b"w\0".as_ptr()));
        dup2(saved, 1);
        close(saved);

        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        buf
    }
}

fn load_c() -> Library {
    unsafe { Library::new(C_LIB).expect("failed to load C lib") }
}

fn load_rust() -> Library {
    unsafe { Library::new(rust_lib()).expect("failed to load Rust lib") }
}

#[test]
fn test_print_hex_char_line() {
    let c_lib = load_c();
    let r_lib = load_rust();

    for &val in &[0i8, 1, 0x0f, 0x10, 0x7f, -1, -128, 42] {
        let v = val as c_char;
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { c_lib.get(b"printHexCharLine").unwrap() };
            capture_stdout(|| unsafe { f(v) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { r_lib.get(b"printHexCharLine").unwrap() };
            capture_stdout(|| unsafe { f(v) })
        };
        assert_eq!(
            c_out, r_out,
            "printHexCharLine({val}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

#[test]
fn test_driver() {
    let c_lib = load_c();
    let r_lib = load_rust();

    for &val in &[0i8, 1, 0x7f, -1, -128, 42, 0x0f] {
        let v = val as c_char;
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { c_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(v) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_char)> =
                unsafe { r_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(v) })
        };
        assert_eq!(
            c_out, r_out,
            "driver({val}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
