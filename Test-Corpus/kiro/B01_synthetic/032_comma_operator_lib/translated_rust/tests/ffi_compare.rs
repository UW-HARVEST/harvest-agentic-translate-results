use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by `f()` using a temp file to avoid pipe blocking.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        let tmpf = libc::tmpfile();
        let tmp_fd = libc::fileno(tmpf);
        let old_stdout = libc::dup(1);
        libc::dup2(tmp_fd, 1);
        f();
        // Flush both Rust and C stdout
        libc::fflush(std::ptr::null_mut());
        use std::io::Write;
        let _ = std::io::stdout().flush();
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        // Read back from temp file
        libc::fseek(tmpf, 0, libc::SEEK_SET);
        let mut r = std::fs::File::from_raw_fd(tmp_fd);
        let mut buf = String::new();
        r.read_to_string(&mut buf).unwrap();
        // Don't fclose tmpf — File owns the fd now
        buf
    }
}

fn c_lib() -> Library {
    unsafe {
        Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so")).unwrap()
    }
}

fn rust_lib() -> Library {
    let path = format!("{}/target/debug/libdriver.so", env!("CARGO_MANIFEST_DIR"));
    unsafe { Library::new(&path).unwrap() }
}

fn call_driver(lib: &Library, x: c_int) -> String {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();
        capture_stdout(|| func(x))
    }
}

#[test]
fn test_driver_positive() {
    let c = c_lib();
    let r = rust_lib();
    for x in [1, 2, 5, 10] {
        let c_out = call_driver(&c, x);
        let r_out = call_driver(&r, x);
        assert_eq!(c_out, r_out, "mismatch for x={x}");
    }
}

#[test]
fn test_driver_zero() {
    let c = c_lib();
    let r = rust_lib();
    let c_out = call_driver(&c, 0);
    let r_out = call_driver(&r, 0);
    assert_eq!(c_out, r_out);
    assert!(c_out.is_empty());
}

#[test]
fn test_driver_negative() {
    let c = c_lib();
    let r = rust_lib();
    for x in [-1, -100] {
        let c_out = call_driver(&c, x);
        let r_out = call_driver(&r, x);
        assert_eq!(c_out, r_out, "mismatch for x={x}");
        assert!(c_out.is_empty());
    }
}
